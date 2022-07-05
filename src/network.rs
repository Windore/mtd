//! A Module defining networking functions for MTD such as syncing with a remote server or running a
//! server. Data transmitted over the network is encrypted.

use std::{fs, io};
use std::io::{Read, Write};
use std::net::{SocketAddr, TcpListener, TcpStream};
use std::path::PathBuf;
use std::time::Duration;

use rand::random;
use serde::{Deserialize, Serialize};

use crate::{Error, Result, TdList};
use crate::network::crypt::{decrypt, encrypt};

/// A config specifying how a `MtdNetMgr` should function. Defining a `save_location` is optional.
/// If it is `None` any `TdList` won't be saved.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Config {
    socket_addr: SocketAddr,
    encryption_password: Vec<u8>,
    timeout: Duration,
    save_location: Option<PathBuf>,
}

impl Config {
    /// Creates a new `Config` with explicit values.
    pub fn new(socket_addr: SocketAddr, encryption_password: Vec<u8>, timeout: Duration, save_location: Option<PathBuf>) -> Self {
        Self { socket_addr, encryption_password, timeout, save_location }
    }
    /// Creates a new `Config` with default values.
    pub fn new_default(encryption_password: Vec<u8>, socket_addr: SocketAddr, save_location: Option<PathBuf>) -> Self {
        Self {
            socket_addr,
            encryption_password,
            timeout: Duration::from_secs(30),
            save_location,
        }
    }
    /// Creates a ´Config` from a JSON string.
    pub fn new_from_json(json: &str) -> Result<Self> {
        Ok(serde_json::from_str(json)?)
    }
    /// Creates a JSON string from the `Config`.
    pub fn to_json(&self) -> Result<String> {
        Ok(serde_json::to_string_pretty(self)?)
    }
    /// Returns the `Config`'s port.
    pub fn socket_addr(&self) -> SocketAddr {
        self.socket_addr
    }
    /// Returns the `Config`'s encryption password.
    pub fn encryption_password(&self) -> &Vec<u8> {
        &self.encryption_password
    }
    /// Returns the `Config`'s timeout duration.
    pub fn timeout(&self) -> Duration {
        self.timeout
    }
    /// Returns the `Config`'s save location.
    pub fn save_location(&self) -> Option<&PathBuf> {
        match &self.save_location {
            None => { None }
            Some(p) => { Some(&p) }
        }
    }
}

/// A struct used for synchronizing `TdList`s between a client and a server over the network. All
/// transmitted data is encrypted using AES GCM. `MtdNetMgr` can act both as a client and as a server.
/// After synchronization data is written to the disk both on the server and the client if the config
/// specifies a `save_location`.
///
/// # Example
///
/// ```
/// use std::net::{IpAddr, Ipv4Addr};
/// use std::thread;
/// use std::time::Duration;
/// use mtd::{Config, MtdNetMgr, TdList, Todo};
///
/// let password = b"Very secure password.";
/// let addr = "127.0.0.1:55995".parse().unwrap();
///
/// // Create a new thread to act as a server.
/// thread::spawn(move || {
///     let mut server_list = TdList::new_server();
///     server_list.add_todo(Todo::new_undated("Todo 1".to_string()));
///
///     let conf = Config::new_default(password.to_vec(), addr, None);
///     let mut server_mgr = MtdNetMgr::new(server_list, &conf);
///
///     server_mgr.server_listening_loop().unwrap();
/// });
///
/// // Give the server some time to bind to a port etc.
/// thread::sleep(Duration::from_millis(500));
///
/// let mut client_list = TdList::new_client();///
///
/// let conf = Config::new_default(password.to_vec(), addr, None);
/// let mut client_mgr = MtdNetMgr::new(client_list, &conf);
/// client_mgr.client_sync().unwrap();
///
/// let client_list = client_mgr.td_list();
/// assert!(client_list.todos().contains(&&Todo::new_undated("Todo 1".to_string())));
/// ```
pub struct MtdNetMgr<'a> {
    td_list: TdList,
    config: &'a Config,
}

impl<'a> MtdNetMgr<'a> {
    /// Creates a new `MtdNetMgr`.
    pub fn new(td_list: TdList, config: &'a Config) -> Self {
        Self { td_list, config }
    }

    /// Returns the contained `TdList`.
    pub fn td_list(self) -> TdList {
        self.td_list
    }

    /// Connects to a server and synchronizes the local `TdList` with a server. Writes the local
    /// `TdList` if the initialization `Config` defined a `save_location`.
    ///
    /// # Panics
    ///
    /// If the `TdList` is a server list.
    pub fn client_sync(&mut self) -> Result<()> {
        if self.td_list.server {
            panic!("Cannot start a client sync with a server TdList");
        }

        let mut stream = TcpStream::connect(self.config.socket_addr())?;

        stream.set_read_timeout(Some(self.config.timeout()))?;
        stream.set_write_timeout(Some(self.config.timeout()))?;

        // Send random data to the server to verify that the server is authentic.
        let random_auth_data: [u8; 8] = random();
        self.write_encrypted(&mut stream, &random_auth_data)?;

        // Server responds with a session id and the previous random data.
        let msg = self.read_decrypted(&mut stream)?;
        if msg.len() < 16 {
            return Err(Error::AuthFailed);
        }

        // set session id
        let sid: [u8; 8] = (&msg[..8]).try_into().unwrap();
        let auth_data: &[u8] = &msg[8..];

        // Check random data
        if auth_data != random_auth_data {
            return Err(Error::AuthFailed);
        }

        // Send read command to server to verify our authenticity.
        self.write_encrypted(&mut stream, &[&sid, b"read".as_slice()].concat())?;

        // Server sends its TdList, sync with that list
        let msg = self.read_check_decrypted(&mut stream, &sid)?;
        let mut server = TdList::new_from_json(&String::from_utf8_lossy(&msg))?;

        self.td_list.sync(&mut server);

        // send the synced list back to the server
        self.write_encrypted(&mut stream, &[&sid, server.to_json()?.as_bytes()].concat())?;

        // Verify that the server actually got its list.
        let msg = self.read_check_decrypted(&mut stream, &sid)?;

        if msg == b"ok" {
            Ok(())
        } else {
            Err(Error::Unknown)
        }
    }

    /// Creates a loop which handles incoming sync connections. Note that each connection is handled in
    /// the same thread sequentially so only one connection can be processed at a time. Writes the local
    /// `TdList` if the initialization `Config` defined a `save_location`.
    ///
    /// # Panics
    ///
    /// If the `TdList` is a client list.
    pub fn server_listening_loop(&mut self) -> io::Result<()> {
        if !self.td_list.server {
            panic!("Cannot start a server loop with a client TdList");
        }

        let tcp = TcpListener::bind(self.config.socket_addr())?;

        for stream in tcp.incoming() {
            match self.handle_stream(stream) {
                Err(e) => {
                    eprintln!("Error occurred: {}", e)
                }
                Ok(_) => {}
            }
        }

        Ok(())
    }

    fn handle_stream(&mut self, stream: io::Result<TcpStream>) -> Result<()> {
        let mut stream = stream?;

        stream.set_read_timeout(Some(self.config.timeout()))?;
        stream.set_write_timeout(Some(self.config.timeout()))?;

        // Random session id for the sync exchange.
        let sid: [u8; 8] = random();

        // First the client sends some random data in an encrypted form to the server.
        let random_auth_data = self.read_decrypted(&mut stream)?;
        // The server sends the data back with a new session id attached.
        self.write_encrypted(&mut stream, &[&sid, random_auth_data.as_slice()].concat())?;

        // Client sends a command to the server.
        let msg = self.read_check_decrypted(&mut stream, &sid)?;

        // Verify that the request is a read request. This just verifies that the client has the right
        // encryption password.
        if msg == b"read" {
            // Respond with the server TdList
            self.write_encrypted(&mut stream, &[&sid, self.td_list.to_json()?.as_bytes()].concat())?;
        } else {
            println!("Client from {} didn't try to read server items. Stopping connection. This is probably a bad sign.", stream.peer_addr()?);
            return Ok(());
        }

        // Client sends a response with a new synced TdList for the server.
        let msg = self.read_check_decrypted(&mut stream, &sid)?;
        let json_string = String::from_utf8_lossy(&msg).to_string();
        self.td_list = TdList::new_from_json(&json_string)?;

        if let Some(path) = self.config.save_location() {
            fs::write(path, &json_string)?;
        }

        // Send ok to the client to verify that everything went right.
        self.write_encrypted(&mut stream, &[&sid, b"ok".as_slice()].concat())?;

        Ok(())
    }

    /// Encrypts and writes a message to a `TcpStream`.
    fn write_encrypted(&self, stream: &mut TcpStream, content: &[u8]) -> Result<()> {
        let enc = encrypt(content, &self.config.encryption_password())?;
        let len = enc.len() as u32;
        let len_header = len.to_le_bytes();
        stream.write(&len_header)?;
        stream.write(&enc)?;
        Ok(())
    }

    /// Reads a message from a `TcpStream` and decrypts it.
    fn read_decrypted(&self, stream: &mut TcpStream) -> Result<Vec<u8>> {
        let mut msg_len_header = [0u8; 4];
        stream.read_exact(&mut msg_len_header)?;
        let len = u32::from_le_bytes(msg_len_header);
        let mut encrypted_msg = vec![0u8; len as usize];
        stream.read_exact(&mut encrypted_msg)?;
        decrypt(&encrypted_msg, &self.config.encryption_password())
    }

    /// Reads a message from a `TcpStream` and decrypts it. Checks the message's session id and returns
    /// the message without a session id.
    fn read_check_decrypted(&self, stream: &mut TcpStream, correct_sid: &[u8; 8]) -> Result<Vec<u8>> {
        MtdNetMgr::check_sid(correct_sid, &self.read_decrypted(stream)?).map(|l| l.to_vec())
    }

    /// Checks if a message contains a valid session id. Returns the message without the session id
    /// if the session id is correct. Otherwise returns an Err.
    fn check_sid<'b>(correct_sid: &[u8; 8], msg_with_sid: &'b [u8]) -> Result<&'b [u8]> {
        if msg_with_sid.len() >= 8 && &msg_with_sid[..8] == correct_sid {
            Ok(&msg_with_sid[8..])
        } else {
            Err(Error::AuthFailed)
        }
    }
}

#[cfg(test)]
mod network_tests {
    use std::{env, fs, thread};
    use std::path::Path;
    use std::time::Duration;

    use crate::{Config, TdList, Todo};
    use crate::network::MtdNetMgr;

    #[test]
    #[should_panic]
    fn mtd_net_mgr_panics_if_server_listener_ran_with_client_td_list() {
        let conf = Config::new("127.0.0.1:55996".parse().unwrap(), Vec::new(), Duration::from_secs(30), None);
        let _ = MtdNetMgr::new(TdList::new_client(), &conf)
            .server_listening_loop();
    }

    #[test]
    #[should_panic]
    fn mtd_net_mgr_panics_if_client_sync_ran_with_server_td_list() {
        let conf = Config::new("127.0.0.1:55996".parse().unwrap(), Vec::new(), Duration::from_secs(30), None);
        let _ = MtdNetMgr::new(TdList::new_server(), &conf)
            .client_sync();
    }

    // This test tests more than one thing, but I believe it to be rather useful. Running more than
    // one test takes more time and this test (and its sub-parts) also depends on external state (IO).
    #[test]
    fn mtd_net_mgr_syncs_correctly() {
        let mut client = TdList::new_client();
        let mut server = TdList::new_server();

        server.add_todo(Todo::new_undated("Todo 1".to_string()));

        // Sync once to set "Todo 1" for both client and server.
        server.sync(&mut client);

        server.get_todo_mut(0).unwrap().set_body("New Todo 1".to_string());
        server.add_todo(Todo::new_undated("Todo 2".to_string()));

        client.add_todo(Todo::new_undated("Todo 3".to_string()));

        let client_conf = Config::new("127.0.0.1:55997".parse().unwrap(), b"hunter42".to_vec(), Duration::from_secs(30), None);
        let mut client_mgr = MtdNetMgr::new(client, &client_conf);

        thread::spawn(move || {
            let server_path = env::temp_dir().join(Path::new("mtd-server-write-test-file"));
            let server_conf = Config::new("127.0.0.1:55997".parse().unwrap(), b"hunter42".to_vec(), Duration::from_secs(30), Some(server_path.clone()));
            let mut server_mgr = MtdNetMgr::new(server, &server_conf);
            server_mgr.server_listening_loop().unwrap();
        });

        thread::sleep(Duration::from_millis(500));

        client_mgr.client_sync().unwrap();

        let client = client_mgr.td_list();

        assert_eq!(client.todos().len(), 3);
        assert!(client.todos().contains(&&Todo::new_undated("New Todo 1".to_string())));
        assert!(client.todos().contains(&&Todo::new_undated("Todo 2".to_string())));
        assert!(client.todos().contains(&&Todo::new_undated("Todo 3".to_string())));

        let server_path = env::temp_dir().join(Path::new("mtd-server-write-test-file"));
        let server = TdList::new_from_json(&fs::read_to_string(server_path).unwrap()).unwrap();

        assert_eq!(server.todos().len(), 3);
        assert!(server.todos().contains(&&Todo::new_undated("New Todo 1".to_string())));
        assert!(server.todos().contains(&&Todo::new_undated("Todo 2".to_string())));
        assert!(server.todos().contains(&&Todo::new_undated("Todo 3".to_string())));
    }
}

/// Module containing functionality for encrypting/decrypting messages used for secure network
/// communication. Data is encrypted with AES-GCM. The encryption key is generated from a password
/// using Argon2. For network communications, session ids should be used in addition to encrypting
/// data.
mod crypt {
    use aes_gcm::{Aes256Gcm, Key, Nonce};
    use aes_gcm::aead::{Aead, NewAead};
    use argon2::Argon2;
    use rand::random;

    use crate::network::Error;

    /// Encrypts a given byte array with the given password.
    pub fn encrypt(msg: &[u8], passwd: &[u8]) -> Result<Vec<u8>, Error> {
        let key_salt: [u8; 16] = random();
        let argon2 = Argon2::default();

        let mut secret_passwd_hash: [u8; 32] = [0; 32];
        argon2.hash_password_into(passwd, &key_salt, &mut secret_passwd_hash).map_err(|_| Error::EncryptingFailed)?;
        let encryption_key = Key::from_slice(&secret_passwd_hash);

        let cipher = Aes256Gcm::new(encryption_key);

        // Random 96-bits for nonce.
        let nonce_bits: [u8; 12] = random();
        let nonce = Nonce::from_slice(nonce_bits.as_slice());

        let mut ciphertext = cipher.encrypt(nonce, msg).map_err(|_| Error::EncryptingFailed)?;

        let mut result = Vec::new();

        result.extend_from_slice(&key_salt);
        result.extend_from_slice(&nonce_bits);
        result.append(&mut ciphertext);

        Ok(result)
    }

    /// Decrypts a given ciphertext with the given password.
    pub fn decrypt(ciphertext: &[u8], passwd: &[u8]) -> Result<Vec<u8>, Error> {
        let key_salt = &ciphertext[0..16];
        let argon2 = Argon2::default();

        let mut secret_passwd_hash: [u8; 32] = [0; 32];
        argon2.hash_password_into(passwd, key_salt, &mut secret_passwd_hash).map_err(|_| Error::DecryptingFailed)?;
        let decryption_key = Key::from_slice(&secret_passwd_hash);

        let cipher = Aes256Gcm::new(decryption_key);

        let nonce_bits = &ciphertext[16..28];
        let nonce = Nonce::from_slice(nonce_bits);

        Ok(cipher.decrypt(nonce, &ciphertext[28..]).map_err(|_| Error::DecryptingFailed)?)
    }

    #[cfg(test)]
    mod tests {
        use crate::network::crypt::{decrypt, encrypt};

        #[test]
        fn decrypting_encrypted_returns_original() {
            let msg = b"A message to keep secure.";
            let ps = b"Very secure passwd";

            let ct = encrypt(msg, ps).unwrap();

            assert_eq!(decrypt(&ct, ps).unwrap(), msg);
        }

        #[test]
        fn encrypting_same_msg_with_same_password_returns_different_ciphertext() {
            let msg = b"A message to keep secure.";
            let ps = b"Very secure passwd";

            let mut ciphertexts = Vec::new();

            for _ in 1..3 {
                let ct = encrypt(msg, ps).unwrap();
                assert!(!ciphertexts.contains(&ct));
                ciphertexts.push(ct);
            }
        }

        #[test]
        fn decrypting_with_incorrect_passwd_fails() {
            let msg = b"A message to keep secure.";
            let ps = b"Very secure passwd";

            let ct = encrypt(msg, ps).unwrap();

            assert!(decrypt(&ct, b"Incorrect passwd").is_err());
        }

        #[test]
        fn decrypting_with_invalid_ciphertext_fails() {
            let msg = b"A message to keep secure.";
            let ps = b"Very secure passwd";

            let mut ct = encrypt(msg, ps).unwrap();
            ct.push(14);
            ct.push(36);
            ct.push(122);

            assert!(decrypt(&ct, ps).is_err());
        }
    }
}
