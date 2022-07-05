use std::{fs, io, process};
use std::io::Write;
use std::net::SocketAddr;
use std::path::PathBuf;

use clap::{ArgEnum, Parser, Subcommand};

use mtd::{Config, Error, Result, TdList};

#[derive(Parser)]
#[clap(author, version, about, long_about = None)]
struct CliArgs {
    #[clap(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Shows specified items
    Show {
        /// Type of items to show.
        #[clap(arg_enum, value_parser, long, short)]
        item_type: Option<ItemType>,
        /// Weekday to show
        #[clap(arg_enum, value_parser, long, short, group = "show_days")]
        weekday: Option<Weekday>,
        /// Show entire week starting from today
        #[clap(value_parser, long, group = "show_days")]
        week: bool,
    },
    /// Adds a new item
    Add {
        /// Type of item to add
        #[clap(arg_enum, value_parser)]
        item_type: ItemType,
        /// Body of the item
        #[clap(value_parser)]
        body: String,
        /// Weekday(s) of the item
        #[clap(arg_enum, value_parser)]
        weekdays: Vec<Weekday>,
    },
    /// Removes an item
    Remove {
        /// Type of item to remove
        #[clap(arg_enum, value_parser)]
        item_type: ItemType,
        /// Id of the item to remove
        #[clap(value_parser)]
        id: u64,
    },
    /// Sets the value(s) of an item
    Set {
        /// Type of item to set the value(s) of
        #[clap(arg_enum, value_parser)]
        item_type: ItemType,
        /// Id of the item to set the value(s) of
        #[clap(value_parser)]
        id: u64,
        /// Set the body of the item
        #[clap(value_parser, long, short)]
        body: Option<String>,
        /// Set the weekday(s) of the item
        #[clap(arg_enum, value_parser, long, short)]
        weekdays: Vec<Weekday>,
    },
    /// Synchronizes local items with a server
    Sync,
    /// Runs mtd as a server
    Server,
    /// Re-initializes mtd
    /// (WARNING! This will completely delete all saved items!)
    ReInit
}

#[derive(Copy, Clone, PartialEq, Eq, PartialOrd, Ord, ArgEnum)]
enum ItemType {
    Todo,
    Task,
}

// Define custom weekday for clap to parse weekdays.
#[derive(Copy, Clone, PartialEq, Eq, PartialOrd, Ord, ArgEnum)]
enum Weekday {
    Mon,
    Tue,
    Wed,
    Thu,
    Fri,
    Sat,
    Sun,
}

impl Into<chrono::Weekday> for Weekday {
    fn into(self) -> chrono::Weekday {
        match self {
            Weekday::Mon => { chrono::Weekday::Mon }
            Weekday::Tue => { chrono::Weekday::Tue }
            Weekday::Wed => { chrono::Weekday::Wed }
            Weekday::Thu => { chrono::Weekday::Thu }
            Weekday::Fri => { chrono::Weekday::Fri }
            Weekday::Sat => { chrono::Weekday::Sat }
            Weekday::Sun => { chrono::Weekday::Sun }
        }
    }
}

fn main() {
    if let Err(e) = MtdApp::run() {
        eprintln!("{}", e);
        process::exit(1);
    } else {
        process::exit(0);
    }
}

struct MtdApp {
    conf: Config,
    list: TdList,
}

impl MtdApp {
    /// Initializes a new MtdApp. Reads/creates config and saved items.
    fn new() -> Result<Self> {
        let config_path = MtdApp::config_path()?;
        let conf;

        if config_path.exists() {
            conf = Config::new_from_json(&fs::read_to_string(config_path)?)?;
        } else {
            conf = MtdApp::create_new_config()?;
        }

        let list;

        // It is possible that a save_location has not been defined which needs to be checked before
        // checking if the path even exists.
        if let Some(list_path) = conf.save_location() {
            if list_path.exists() {
                list = TdList::new_from_json(
                    &fs::read_to_string(
                        list_path
                    )?
                )?;
            } else {
                list = MtdApp::create_new_list()?;
            }
        } else {
            list = MtdApp::create_new_list()?;
        }

        Ok(Self {
            conf,
            list,
        })
    }

    /// Creates a new TdList as a server or a client depending on user input.
    fn create_new_list() -> Result<TdList> {
        println!("Initializing.");

        let mut buffer = String::new();
        let stdin = io::stdin();
        let mut stdout = io::stdout();

        loop {
            print!("Initialize as a server or a client (s/c)? ");
            stdout.flush()?;
            buffer.clear();
            stdin.read_line(&mut buffer)?;
            buffer = buffer.to_lowercase().trim().to_string();

            if &buffer != "s" && &buffer != "c" {
                eprintln!("Invalid option.");
                continue;
            }
            break;
        }

        if &buffer == "c" {
            Ok(TdList::new_client())
        } else {
            Ok(TdList::new_server())
        }
    }

    /// Returns the path to the config.
    fn config_path() -> Result<PathBuf> {
        Ok(dirs::config_dir().ok_or(Error::Unknown)?.join("mtd/conf.json"))
    }

    /// Returns the path to the default save location.
    fn default_save_path() -> Result<PathBuf> {
        Ok(dirs::data_dir().ok_or(Error::Unknown)?.join("mtd/data.json"))
    }

    /// Initializes a new config and writes it to a file.
    fn create_new_config() -> Result<Config> {
        println!("Creating a new config.");

        let config_path = MtdApp::config_path()?;

        let stdin = io::stdin();
        let mut stdout = io::stdout();

        let mut socket_addr = String::new();

        loop {
            print!("Input server socket address (IP:PORT): ");
            stdout.flush()?;
            socket_addr.clear();
            stdin.read_line(&mut socket_addr)?;
            socket_addr = socket_addr.trim().to_string();

            if socket_addr.parse::<SocketAddr>().is_err() {
                eprintln!("Cannot parse '{}' to socket address.", socket_addr);
                continue;
            }
            break;
        }

        println!("Note! Encryption password is stored in cleartext but obfuscated locally.");

        let mut encryption_passwd = String::new();
        let mut encryption_passwd_again = String::new();

        loop {
            print!("Input encryption password: ");
            stdout.flush()?;
            encryption_passwd.clear();
            stdin.read_line(&mut encryption_passwd)?;
            encryption_passwd = encryption_passwd.trim().to_string();

            print!("Input encryption password again: ");
            stdout.flush()?;
            encryption_passwd_again.clear();
            stdin.read_line(&mut encryption_passwd_again)?;
            encryption_passwd_again = encryption_passwd_again.trim().to_string();

            if encryption_passwd != encryption_passwd_again {
                eprintln!("Passwords do not match.");
                continue;
            } else if encryption_passwd.is_empty() {
                eprintln!("Password cannot be empty.");
                continue;
            }
            break;
        }

        let conf = Config::new_default(
            encryption_passwd.into_bytes(),
            socket_addr.parse().unwrap(),
            Some(MtdApp::default_save_path()?),
        );

        if let Some(conf_dir) = config_path.parent() {
            fs::create_dir_all(conf_dir)?;
        }
        fs::write(&config_path, conf.to_json()?)?;

        Ok(conf)
    }

    /// Runs the mtd cli app.
    fn run() -> Result<()> {
        let cli = CliArgs::parse();
        let mut app = MtdApp::new()?;

        match &cli.command {
            Commands::Show { item_type, weekday, week } => {
                app.show(item_type, weekday, week);
            }
            Commands::Add { item_type, weekdays, body } => {
                app.add(item_type, weekdays, body);
            }
            Commands::Remove { item_type, id } => {
                app.remove(item_type, id)?;
            }
            Commands::Set { item_type, id, body, weekdays } => {
                app.set(item_type, id, body, weekdays)?;
            }
            Commands::Sync {} => {
                app.sync()?;
            }
            Commands::Server {} => {
                app.server()?
            }
            Commands::ReInit {} => {
                app.re_init()?;
            }
        }

        if let Some(path) = app.conf.save_location() {
            if !path.exists() {
                if let Some(parent) = path.parent() {
                    fs::create_dir_all(parent)?;
                }
                fs::write(path, app.list.to_json()?)?;
            }
        }

        Ok(())
    }

    fn show(&self, item_type: &Option<ItemType>, weekday: &Option<Weekday>, week: &bool) {}

    fn add(&mut self, item_type: &ItemType, weekdays: &Vec<Weekday>, body: &String) {}

    fn remove(&mut self, item_type: &ItemType, id: &u64) -> Result<()> {
        Ok(())
    }

    fn set(&mut self, item_type: &ItemType, id: &u64, body: &Option<String>, weekdays: &Vec<Weekday>) -> Result<()> {
        Ok(())
    }

    fn sync(&mut self) -> Result<()> {
        Ok(())
    }

    fn server(&mut self) -> Result<()> {
        Ok(())
    }

    fn re_init(&mut self) -> Result<()> {
        let stdin = io::stdin();
        let mut stdout = io::stdout();

        let mut buffer = String::new();

        loop {
            print!("This will delete all items and erase the config. Proceed (y/n)? ");
            stdout.flush()?;
            buffer.clear();
            stdin.read_line(&mut buffer)?;
            buffer = buffer.to_lowercase().trim().to_string();

            if &buffer != "y" && &buffer != "n" {
                eprintln!("Invalid option.");
                continue;
            }
            break;
        }

        if &buffer == "n" {
            println!("Abort!");
            return Ok(());
        }

        self.conf = MtdApp::create_new_config()?;
        self.list = MtdApp::create_new_list()?;

        Ok(())
    }
}