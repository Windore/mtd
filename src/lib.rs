//! A crate defining the internal functionality of MTD. Works as the core for MTD CLI and Android app.
//! This crate can be used to crate MTD compatible applications.
//!
//! MTD is the newest Todo app in my long lasting line of different Todo app, but this time hopefully
//! the final addition. MTD supports synchronization using a self-hosted server as in one server supports
//! only one user.
//!
//! # Example
//!
//! ```
//! use chrono::Weekday;
//! use mtd::{Task, TdList, Todo};
//!
//! // Creates a new TdList which is a list that is used for containing Todos and Tasks.
//! let mut client = TdList::new_client();
//!
//! // Adds a new Todo that should be done the next Friday.
//! client.add_todo(Todo::new_dated("Install MTD".to_string(), Weekday::Fri));
//!
//! // Adds a new Task that should be done every Wednesday and Saturday.
//! client.add_task(Task::new("Clean the house.".to_string(), vec![Weekday::Wed, Weekday::Sat]));
//!
//! // This TdList should be the one got from the server. It usually shouldn't be modified directly
//! // because all modifications made on the client will be synced to the server.
//! let mut server = TdList::new_server();
//!
//! // The new added items will be *copied* to the server.
//! client.sync(&mut server);
//!
//! assert!(server.todos().contains(&&Todo::new_dated("Install MTD".to_string(), Weekday::Fri)));
//! assert!(server.tasks().contains(&&Task::new("Clean the house.".to_string(), vec![Weekday::Wed, Weekday::Sat])));
//!
//! // Modifications such as setting a Todo done are also copied to the server.
//! client.get_todo_mut(0).unwrap().set_done(true);
//! assert_ne!(client.todos()[0].done(), server.todos()[0].done());
//!
//! client.sync(&mut server);
//! assert_eq!(client.todos()[0].done(), server.todos()[0].done());
//! ```

#![warn(missing_docs)]

extern crate core;

use std::borrow::BorrowMut;
use std::collections::HashMap;
use std::fmt::{Debug, Display, Formatter};
use std::io;

use chrono::{Datelike, Local, NaiveDate, Weekday};
use rand::random;
use serde::{Deserialize, Serialize};

pub use network::MtdNetMgr;

mod network;
// Methods ending with _wtd are used for unit testing and internal implementations. They allow
// supplying today with any date.

/// Custom errors returned by this crate. Some errors wrap existing errors.
#[derive(Debug)]
pub enum Error {
    /// Indicates that no `Todo` with the given `id` exists.
    NoTodoWithGivenIdErr(u64),
    /// Indicates that no `Task` with the given `id` exists.
    NoTaskWithGivenIdErr(u64),
    /// Indicates that encrypting data failed.
    EncryptingErr,
    /// Indicates that decrypting data failed. The two common reasons for this error are incorrect
    /// passwords or tampered ciphertexts.
    DecryptingErr,
    /// Indicates that something IO related failed.
    IoErr(io::Error),
    /// Indicates that serialization failed.
    SerdeErr(serde_json::Error),
    /// Indicates that authentication of the client/server failed.
    AuthErr,
    /// Writing `TdList` on a server failed. Server didn't respond with a success signal.
    ServerWriteFailed,
}

impl Display for Error {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Error::NoTodoWithGivenIdErr(id) => {
                write!(f, "No Todo with the given id: \"{}\" found.", id)
            }
            Error::NoTaskWithGivenIdErr(id) => {
                write!(f, "No Task with the given id: \"{}\" found.", id)
            }
            Error::EncryptingErr => {
                write!(f, "Encrypting data failed.")
            }
            Error::DecryptingErr => {
                write!(f, "Decrypting data failed.")
            }
            Error::IoErr(e) => {
                write!(f, "{}", e)
            }
            Error::SerdeErr(e) => {
                write!(f, "{}", e)
            }
            Error::AuthErr => {
                write!(f, "Authentication failed.")
            }
            Error::ServerWriteFailed => {
                write!(f, "Writing data to server failed.")
            }
        }
    }
}

impl From<io::Error> for Error {
    fn from(e: io::Error) -> Self {
        Error::IoErr(e)
    }
}

impl From<serde_json::Error> for Error {
    fn from(e: serde_json::Error) -> Self {
        Error::SerdeErr(e)
    }
}

impl std::error::Error for Error {}

/// Gets the date that represents the upcoming weekday. Given tomorrow’s weekday, this should return
/// tomorrows date. Today is represented by the current weekday.
fn weekday_to_date(weekday: Weekday, mut today: NaiveDate) -> NaiveDate {
    loop {
        if today.weekday() == weekday {
            return today;
        }
        today = today.succ();
    }
}

/// Represents a one-time task to be done at a specific date. The date is specified as a weekday
/// from now. If no weekday is given, the current weekday will be used. After the given weekday, the
/// `Todo` will show up for the current day.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Todo {
    body: String,
    date: NaiveDate,
    id: u64,
    done: Option<NaiveDate>,
    sync_id: u64,
    state: ItemState,
}

impl Todo {
    /// Creates a new `Todo` that shows up to be done for the current day.
    pub fn new_undated(body: String) -> Todo {
        Todo {
            body,
            date: Local::today().naive_local(),
            id: 0,
            done: None,
            sync_id: random(),
            state: ItemState::Unchanged,
        }
    }

    /// Creates a new `Todo` that shows up to be done at a specific weekday.
    pub fn new_dated(body: String, weekday: Weekday) -> Todo {
        Todo {
            body,
            date: weekday_to_date(weekday, Local::today().naive_local()),
            id: 0,
            done: None,
            sync_id: random(),
            state: ItemState::Unchanged,
        }
    }

    // Used for unit testing with non-today dependant date
    #[cfg(test)]
    fn new_specific_date(body: String, date: NaiveDate) -> Todo {
        Todo {
            body,
            date,
            id: 0,
            done: None,
            sync_id: random(),
            state: ItemState::Unchanged,
        }
    }

    /// Returns `true` if the `Todo` is for a given date.
    ///
    /// # Example
    ///
    /// ```
    /// use chrono::{Datelike, Local};
    /// use mtd::Todo;
    ///
    /// let todo_for_today = Todo::new_undated("I am for today".to_string());
    ///
    /// assert!(todo_for_today.for_date(Local::today().naive_local()));
    ///
    /// let todo_for_tomorrow = Todo::new_dated("I am for tomorrow".to_string(), Local::today().naive_local().succ().weekday());
    ///
    /// assert!(!todo_for_tomorrow.for_date(Local::today().naive_local()));
    /// assert!(todo_for_tomorrow.for_date(Local::today().naive_local().succ()));
    /// ```
    pub fn for_date(&self, date: NaiveDate) -> bool {
        self.for_date_wtd(date, Local::today().naive_local())
    }

    fn for_date_wtd(&self, date: NaiveDate, today: NaiveDate) -> bool {
        date >= self.date && (date == today || self.date > today)
    }

    /// Gets the `body` of the `Todo`.
    pub fn body(&self) -> &str {
        &self.body
    }

    /// Gets the weekday of the `Todo`.
    pub fn weekday(&self) -> Weekday {
        self.date.weekday()
    }

    /// Gets the `id` of the `Todo`.
    pub fn id(&self) -> u64 {
        self.id
    }

    /// Sets the `body` of the `Todo`.
    pub fn set_body(&mut self, body: String) {
        self.body = body;
        self.state = ItemState::Changed;
    }

    /// Sets the weekday of the `Todo`.
    pub fn set_weekday(&mut self, weekday: Weekday) {
        self.date = weekday_to_date(weekday, Local::today().naive_local());
        self.state = ItemState::Changed;
    }

    /// Returns `true` if the `Todo` is done.
    pub fn done(&self) -> bool {
        self.done.is_some()
    }

    /// Sets the done state of the `Todo`.
    pub fn set_done(&mut self, done: bool) {
        self.set_done_wtd(done, Local::today().naive_local());
    }

    fn set_done_wtd(&mut self, done: bool, today: NaiveDate) {
        if done {
            self.done = Some(today);
        } else {
            self.done = None;
        }
        self.state = ItemState::Changed;
    }

    fn set_id(&mut self, id: u64) {
        self.id = id;
    }

    /// Returns `true` if the `Todo` can be removed. A `Todo` can be removed one day after its
    /// completion.
    pub fn can_remove(&self) -> bool {
        self.can_remove_wtd(Local::today().naive_local())
    }

    fn can_remove_wtd(&self, today: NaiveDate) -> bool {
        if let Some(done_date) = self.done {
            today > done_date
        } else {
            false
        }
    }
}

impl Display for Todo {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{} (ID: {})", self.body, self.id)
    }
}

impl PartialEq for Todo {
    fn eq(&self, other: &Self) -> bool {
        self.body == other.body &&
            self.date == other.date &&
            self.done == other.done
    }
}

/// Represents a reoccurring task for the given weekday(s).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Task {
    body: String,
    weekdays: Vec<Weekday>,
    done_map: HashMap<Weekday, NaiveDate>,
    id: u64,
    state: ItemState,
    sync_id: u64,
}

impl Task {
    /// Creates a new task for the given weekday(s).
    ///
    /// # Panics
    ///
    /// If the given weekdays list is empty.
    pub fn new(body: String, weekdays: Vec<Weekday>) -> Task {
        if weekdays.is_empty() {
            panic!("Cannot create a task without specifying at least one weekday.")
        }
        Task { body, weekdays, id: 0, done_map: HashMap::new(), sync_id: random(), state: ItemState::Unchanged }
    }

    /// Gets the `body` of the `Task`.
    pub fn body(&self) -> &str {
        &self.body
    }

    /// Gets the `weekdays` of the `Task`. Note that duplicate weekdays are allowed.
    pub fn weekdays(&self) -> &Vec<Weekday> {
        &self.weekdays
    }

    /// Gets the `id` of the `Task`.
    pub fn id(&self) -> u64 {
        self.id
    }

    /// Sets the `body` of the `Task`.
    pub fn set_body(&mut self, body: String) {
        self.body = body;
        self.state = ItemState::Changed;
    }

    fn set_id(&mut self, id: u64) {
        self.id = id;
    }

    /// Sets the `weekdays` of the `Task`.
    pub fn set_weekdays(&mut self, weekdays: Vec<Weekday>) {
        self.weekdays = weekdays;
        self.state = ItemState::Changed;
    }

    /// Adds a weekday to the weekdays list.
    pub fn add_weekday(&mut self, weekday: Weekday) {
        // It doesn't matter if there are duplicate weekdays.
        self.weekdays.push(weekday);
        self.state = ItemState::Changed;
    }

    /// Removes a weekday from the weekdays list. Removes all duplicates as well.
    ///
    /// # Example
    ///
    /// ```
    /// use chrono::Weekday;
    /// use mtd::Task;
    ///
    /// let mut task = Task::new("Test task".to_string(), vec![Weekday::Mon, Weekday::Tue, Weekday::Wed]);
    /// task.remove_weekday(Weekday::Wed);
    ///
    /// // Removing a weekday that isn't listed does nothing.
    /// task.remove_weekday(Weekday::Fri);
    ///
    /// assert!(task.weekdays().contains(&Weekday::Mon));
    /// assert!(task.weekdays().contains(&Weekday::Tue));
    /// // Doesn't contain wed anymore
    /// assert!(!task.weekdays().contains(&Weekday::Wed));
    /// ```
    pub fn remove_weekday(&mut self, removed_wd: Weekday) {
        let mut new_weekdays = Vec::new();

        for wd in &self.weekdays {
            if wd != &removed_wd {
                new_weekdays.push(wd.clone());
            }
        }

        self.set_weekdays(new_weekdays);
    }

    /// Returns `true` if the `Task` is for a given date.
    ///
    /// # Example
    ///
    /// ```
    /// use chrono::{NaiveDate, Weekday};
    /// use mtd::Task;
    ///
    /// let task = Task::new("Task".to_string(), vec![Weekday::Fri, Weekday::Sun]);
    ///
    /// assert!(task.for_date(NaiveDate::from_ymd(2022, 6, 10))); // 2022-6-10 is a Friday
    /// assert!(!task.for_date(NaiveDate::from_ymd(2022, 6, 11))); // Saturday
    /// assert!(task.for_date(NaiveDate::from_ymd(2022, 6, 12))); // Sunday
    /// ```
    pub fn for_date(&self, date: NaiveDate) -> bool {
        self.weekdays.contains(&date.weekday())
    }

    /// Returns `true` if the `Task` is done for the given date. Always returns `true` if the task
    /// is not for the given the date.
    ///
    /// # Example
    ///
    /// ```
    /// use chrono::{NaiveDate, Weekday};
    /// use mtd::Task;
    ///
    /// let mut task = Task::new("Task".to_string(), vec![Weekday::Mon, Weekday::Wed, Weekday::Thu]);
    ///
    /// task.set_done(true, NaiveDate::from_ymd(2022, 6, 13));
    /// task.set_done(true, NaiveDate::from_ymd(2022, 6, 16));
    ///
    /// // Done for mon and thu
    /// assert!(task.done(NaiveDate::from_ymd(2022, 6, 13)));
    /// assert!(task.done(NaiveDate::from_ymd(2022, 6, 16)));
    ///
    /// // Not done for wed
    /// assert!(!task.done(NaiveDate::from_ymd(2022, 6, 15)));
    ///
    /// // Not done for the following week's mon/thu
    /// assert!(!task.done(NaiveDate::from_ymd(2022, 6, 20)));
    /// assert!(!task.done(NaiveDate::from_ymd(2022, 6, 23)));
    ///
    /// // Since 2022-6-21 is a tue, the task is done for that date
    /// assert!(task.done(NaiveDate::from_ymd(2022, 6, 21)));
    /// ```
    pub fn done(&self, date: NaiveDate) -> bool {
        if self.for_date(date) {
            if let Some(d) = self.done_map.get(&date.weekday()) {
                return *d >= date;
            }
            return false;
        }
        true
    }


    /// Sets the done state of the `Task` for the given date.
    ///
    /// # Example
    ///
    /// ```
    ///
    /// use chrono::{NaiveDate, Weekday};
    /// use mtd::Task;
    ///
    /// let mut task = Task::new("Task".to_string(), vec![Weekday::Mon]);
    ///
    /// task.set_done(true, NaiveDate::from_ymd(2022, 6, 13));
    /// assert!(task.done(NaiveDate::from_ymd(2022, 6, 13)));
    ///
    /// task.set_done(false, NaiveDate::from_ymd(2022, 6, 13));
    /// assert!(!task.done(NaiveDate::from_ymd(2022, 6, 13)));
    /// ```
    pub fn set_done(&mut self, done: bool, date: NaiveDate) {
        if done {
            self.done_map.insert(date.weekday(), date);
        } else {
            self.done_map.remove(&date.weekday());
        }
    }
}

impl Display for Task {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{} (ID: {})", self.body, self.id)
    }
}

impl PartialEq for Task {
    fn eq(&self, other: &Self) -> bool {
        self.body == other.body &&
            self.weekdays == other.weekdays &&
            self.done_map == other.done_map
    }
}

#[derive(Debug, PartialEq, Copy, Clone, Serialize, Deserialize)]
enum ItemState {
    New,
    Removed,
    Unchanged,
    Changed,
}

trait SyncItem {
    fn set_state(&mut self, state: ItemState);
    fn state(&self) -> ItemState;
    fn set_id(&mut self, id: u64);
    fn sync_id(&self) -> u64;
    fn update_old(&self, old: &mut Self);
}

impl SyncItem for Todo {
    fn set_state(&mut self, state: ItemState) {
        self.state = state;
    }

    fn state(&self) -> ItemState {
        self.state
    }

    fn set_id(&mut self, id: u64) {
        self.id = id;
    }
    fn sync_id(&self) -> u64 {
        self.sync_id
    }

    fn update_old(&self, old: &mut Self) {
        old.body = self.body.clone();
        old.date = self.date.clone();
        old.done = self.done.clone();
    }
}

impl SyncItem for Task {
    fn set_state(&mut self, state: ItemState) {
        self.state = state;
    }

    fn state(&self) -> ItemState {
        self.state
    }

    fn set_id(&mut self, id: u64) {
        self.id = id;
    }
    fn sync_id(&self) -> u64 {
        self.sync_id
    }

    fn update_old(&self, old: &mut Self) {
        old.body = self.body.clone();
        old.weekdays = self.weekdays.clone();
        old.done_map = self.done_map.clone();
    }
}

#[derive(Debug, Serialize, Deserialize)]
struct SyncList<T: SyncItem + Clone> {
    items: Vec<T>,
    server: bool,
}

impl<T: SyncItem + Clone + PartialEq> SyncList<T> {
    fn new(server: bool) -> Self {
        Self {
            items: Vec::new(),
            server,
        }
    }
    fn add(&mut self, mut item: T) {
        item.set_state(ItemState::New);
        self.items.push(item);
    }
    fn mark_removed(&mut self, id: u64) -> Result<(), ()> {
        if id >= self.items.len() as u64 {
            return Err(());
        }
        let item = self.items[id as usize].borrow_mut();

        // Do not allow the removal of items already removed.
        if item.state() == ItemState::Removed {
            return Err(());
        }

        item.set_state(ItemState::Removed);

        // Servers remove the items immediately.
        if self.server {
            self.items.retain(|item| item.state() != ItemState::Removed);
            self.map_indices_to_ids();
        }

        Ok(())
    }
    fn map_indices_to_ids(&mut self) {
        for (new_id, item) in self.items.iter_mut().enumerate() {
            item.set_id(new_id as u64);
        }
    }
    fn items(&self) -> Vec<&T> {
        let mut items = Vec::new();
        for item in &self.items {
            if item.state() != ItemState::Removed {
                items.push(item);
            }
        }

        items
    }
    fn get_item_mut(&mut self, id: u64) -> Option<&mut T> {
        self.items.get_mut(id as usize)
    }
    fn sync_self(&mut self) {
        self.items.retain(|item| item.state() != ItemState::Removed);
        self.map_indices_to_ids();
        for item in self.items.iter_mut() {
            item.set_state(ItemState::Unchanged);
        }
    }
    fn sync(&mut self, other: &mut Self) {
        if self.server && other.server {
            panic!("Both self and other are servers.");
        } else if !self.server && !other.server {
            panic!("Neither self or other is a server.");
        }

        let server_list;
        let client_list;
        if self.server {
            server_list = self;
            client_list = other
        } else {
            server_list = other;
            client_list = self;
        }

        for item in client_list.items.iter_mut() {
            match item.state() {
                ItemState::New => {
                    server_list.add(item.clone());
                }
                ItemState::Removed => {
                    if let Some(s_item) = server_list.get_item_by_sync_id(item.sync_id()) {
                        s_item.set_state(ItemState::Removed);
                    }
                }
                ItemState::Unchanged => {
                    if let Some(s_item) = server_list.get_item_by_sync_id(item.sync_id()) {
                        // If this is false then the item has been modified on the server.
                        if s_item != item {
                            // Update the client item to match the server item.
                            s_item.update_old(item);
                        }
                    } else {
                        item.set_state(ItemState::Removed);
                    }
                }
                ItemState::Changed => {
                    if let Some(s_item) = server_list.get_item_by_sync_id(item.sync_id()) {
                        item.update_old(s_item);
                    } else {
                        // The modified item doesn't exist on the server therefore it needs to be
                        // added.
                        server_list.add(item.clone());
                    }
                }
            }
        }

        for item in server_list.items.iter() {
            if item.state() != ItemState::Removed {
                if client_list.get_item_by_sync_id(item.sync_id()).is_none() {
                    client_list.add(item.clone());
                }
            }
        }

        client_list.sync_self();
        server_list.sync_self();
    }

    fn get_item_by_sync_id(&mut self, sync_id: u64) -> Option<&mut T> {
        self.items.iter_mut().filter(|i| i.sync_id() == sync_id).next()
    }
}

/// A synchronizable list used for containing and managing all `Todo`s and `Task`s. `Todo`s and
/// `Task`s have `id`s that match their `id`s within the `TdList`.
#[derive(Debug, Serialize, Deserialize)]
pub struct TdList {
    todos: SyncList<Todo>,
    tasks: SyncList<Task>,
    server: bool,
}

impl TdList {
    /// Creates a new empty client `TdList`.
    pub fn new_client() -> Self {
        Self { todos: SyncList::new(false), tasks: SyncList::new(false), server: false }
    }

    /// Creates a new empty server `TdList`.
    pub fn new_server() -> Self {
        Self { todos: SyncList::new(true), tasks: SyncList::new(true), server: true }
    }

    /// Creates a ´TdList` from a JSON string.
    pub fn new_from_json(json: &str) -> serde_json::Result<Self> {
        serde_json::from_str(json)
    }

    /// Creates a JSON string from the `TdList`.
    pub fn to_json(&self) -> serde_json::Result<String> {
        serde_json::to_string(self)
    }

    /// Gets all the `Todo`s in the list.
    pub fn todos(&self) -> Vec<&Todo> {
        self.todos.items()
    }

    /// Gets all the `Task`s in the list.
    pub fn tasks(&self) -> Vec<&Task> {
        self.tasks.items()
    }

    /// Adds a `Todo` to the list and updates its id.
    pub fn add_todo(&mut self, mut todo: Todo) {
        todo.set_id(self.todos.items.len() as u64);
        self.todos.add(todo);
    }

    /// Adds a `Task` to the list and updates its id.
    pub fn add_task(&mut self, mut task: Task) {
        task.set_id(self.tasks.items.len() as u64);
        self.tasks.add(task)
    }

    /// Removes the `Todo` that matches the given id. If no `Todo` with the given `id` exists, returns
    /// a `MtdError`.
    pub fn remove_todo(&mut self, id: u64) -> Result<(), Error> {
        self.todos.mark_removed(id).map_err(|_| Error::NoTodoWithGivenIdErr(id))
    }

    /// Removes the `Task` that matches the given id. If no `Task` with the given `id` exists, returns
    /// a `MtdError`.
    pub fn remove_task(&mut self, id: u64) -> Result<(), Error> {
        self.tasks.mark_removed(id).map_err(|_| Error::NoTaskWithGivenIdErr(id))
    }

    /// Returns a mutable reference to a `Todo` by its `id`. If no `Todo` with the given `id` exists
    /// return `None`.
    pub fn get_todo_mut(&mut self, id: u64) -> Option<&mut Todo> {
        self.todos.get_item_mut(id)
    }

    /// Returns a mutable reference to a `Task` by its `id`. If no `Task` with the given `id` exists
    /// return `None`.
    pub fn get_task_mut(&mut self, id: u64) -> Option<&mut Task> {
        self.tasks.get_item_mut(id)
    }

    /// Returns all `Todo`s for a given date that are not yet done.
    pub fn undone_todos_for_date(&self, date: NaiveDate) -> Vec<&Todo> {
        self.undone_todos_for_date_wtd(date, Local::today().naive_local())
    }

    /// Returns all `Todo`s for a given date that are done.
    pub fn done_todos_for_date(&self, date: NaiveDate) -> Vec<&Todo> {
        self.done_todos_for_date_wtd(date, Local::today().naive_local())
    }

    fn undone_todos_for_date_wtd(&self, date: NaiveDate, today: NaiveDate) -> Vec<&Todo> {
        let mut undone_todos = Vec::new();

        for todo in self.todos.items() {
            if todo.for_date_wtd(date, today) && !todo.done() {
                undone_todos.push(todo);
            }
        }

        undone_todos
    }

    fn done_todos_for_date_wtd(&self, date: NaiveDate, today: NaiveDate) -> Vec<&Todo> {
        let mut done_todos = Vec::new();

        for todo in self.todos.items() {
            if todo.for_date_wtd(date, today) && todo.done() {
                done_todos.push(todo);
            }
        }

        done_todos
    }

    /// Returns all `Task`s for a given date that are not yet done.
    pub fn undone_tasks_for_date(&self, date: NaiveDate) -> Vec<&Task> {
        let mut undone_tasks = Vec::new();

        for task in self.tasks.items() {
            if task.for_date(date) && !task.done(date) {
                undone_tasks.push(task);
            }
        }

        undone_tasks
    }

    /// Returns all `Task`s for a given date that are done.
    pub fn done_tasks_for_date(&self, date: NaiveDate) -> Vec<&Task> {
        let mut done_tasks = Vec::new();

        for task in self.tasks.items() {
            if task.for_date(date) && task.done(date) {
                done_tasks.push(task);
            }
        }

        done_tasks
    }

    /// Removes all `Todo`s that are done and at least a day has passed since their completion.
    /// Basically remove all `Todo`s which `Todo.can_remove()` returns `true`. This is called
    /// automatically every sync.
    pub fn remove_old_todos(&mut self) {
        self.remove_old_todos_wtd(Local::today().naive_local());
    }

    fn remove_old_todos_wtd(&mut self, today: NaiveDate) {
        for todo in &mut self.todos.items {
            if todo.can_remove_wtd(today) {
                todo.state = ItemState::Removed;
            }
        }
        if self.server {
            self.todos.items.retain(|todo| todo.state != ItemState::Removed);
        }
    }

    /// Synchronizes the list with itself actually removing items. Synchronizing may change the `id`s
    /// of both `Todo`s and `Task`s. Additionally removes old `Todo`s.
    pub fn self_sync(&mut self) {
        self.remove_old_todos();
        self.todos.sync_self();
        self.tasks.sync_self();
    }

    // This method is only unit tested using Todos which is fine as long as the internal sync impl
    // of todos and tasks is the same because then these tests cover Tasks as well.
    /// Synchronizes the list with another list actually removing items. Synchronizing may change the `id`s
    /// of both `Todo`s and `Task`s. Additionally removes old `Todo`s.
    ///
    /// # Example
    ///
    /// ```
    /// use mtd::{TdList, Todo};
    ///
    /// let mut client = TdList::new_client();
    /// let mut server = TdList::new_server();
    ///
    /// client.add_todo(Todo::new_undated("Todo 1".to_string()));
    ///
    /// server.add_todo(Todo::new_undated("Todo 2".to_string()));
    ///
    /// // New todos are added to both the server and the client.
    /// client.sync(&mut server);
    ///
    /// assert!(client.todos().contains(&&Todo::new_undated("Todo 1".to_string())));
    /// assert!(client.todos().contains(&&Todo::new_undated("Todo 2".to_string())));
    /// assert_eq!(client.todos().len(), 2);
    ///
    /// assert!(server.todos().contains(&&Todo::new_undated("Todo 1".to_string())));
    /// assert!(server.todos().contains(&&Todo::new_undated("Todo 2".to_string())));
    /// assert_eq!(server.todos().len(), 2);
    ///
    /// client.remove_todo(0).unwrap();
    ///
    /// // The removed item gets removed from both the server and the client.
    /// client.sync(&mut server);
    ///
    /// assert!(client.todos().contains(&&Todo::new_undated("Todo 2".to_string())));
    /// assert_eq!(client.todos().len(), 1);
    ///
    /// assert!(server.todos().contains(&&Todo::new_undated("Todo 2".to_string())));
    /// assert_eq!(server.todos().len(), 1);
    ///
    /// client.get_todo_mut(0).unwrap().set_body("New Todo 1".to_string());
    ///
    /// // Modifications are synchronized as well.
    /// client.sync(&mut server);
    ///
    /// assert!(client.todos().contains(&&Todo::new_undated("New Todo 1".to_string())));
    /// assert_eq!(client.todos().len(), 1);
    ///
    /// assert!(server.todos().contains(&&Todo::new_undated("New Todo 1".to_string())));
    /// assert_eq!(server.todos().len(), 1);
    /// ```
    pub fn sync(&mut self, other: &mut Self) {
        self.remove_old_todos();
        other.remove_old_todos();

        self.todos.sync(&mut other.todos);
        self.tasks.sync(&mut other.tasks);
    }
}

#[cfg(test)]
mod tests {
    use chrono::{NaiveDate, Weekday};

    use crate::{Task, TdList, Todo, weekday_to_date};

    // Unit test a private function to remove the need to pass today into the Todo constructor
    #[test]
    fn weekday_to_date_returns_correct_dates() {
        // Today is a Tuesday
        let today = NaiveDate::from_ymd(2022, 6, 7);

        // Tue should return today’s date
        assert_eq!(weekday_to_date(Weekday::Tue, today), today);

        // Wed should return tomorrow’s date
        assert_eq!(weekday_to_date(Weekday::Wed, today), today.succ());

        // Mon should return next weeks monday
        assert_eq!(weekday_to_date(Weekday::Mon, today), NaiveDate::from_ymd(2022, 6, 13));
    }

    #[test]
    fn todo_displays_correctly() {
        let todo = Todo::new_undated("Todo".to_string());
        assert_eq!(todo.to_string(), "Todo (ID: 0)".to_string());
    }

    #[test]
    fn todo_for_date_tests() {
        let todo = Todo::new_specific_date("Friday".to_string(), NaiveDate::from_ymd(2022, 6, 10));

        let today = NaiveDate::from_ymd(2022, 6, 10);

        // The following 5 asserts could each be their own unit test but I'm to lazy to do it so
        // instead I just added some comments explaining the tests

        assert!(todo.for_date_wtd(today, today)); // Todo is for the given date on the same day
        assert!(todo.for_date_wtd(today, today.pred())); // Todo is for the given date before the given date
        assert!(!todo.for_date_wtd(today, today.succ())); // Todo is not for the given date after the given date
        assert!(todo.for_date_wtd(today.succ(), today.succ())); // Todo is for the following date one day after the given date
        assert!(!todo.for_date_wtd(today.succ(), today)); // Todo is not for the following date because it is already for today
    }

    #[test]
    fn todo_can_remove_returns_true_only_after_one_day_from_completion() {
        let mut todo = Todo::new_specific_date("Todo".to_string(), NaiveDate::from_ymd(2022, 4, 25));
        todo.set_done_wtd(true, NaiveDate::from_ymd(2022, 4, 26));

        assert!(!todo.can_remove_wtd(NaiveDate::from_ymd(2022, 4, 26)));
        assert!(todo.can_remove_wtd(NaiveDate::from_ymd(2022, 4, 27)));
        assert!(todo.can_remove_wtd(NaiveDate::from_ymd(2022, 4, 28)));
    }

    #[test]
    #[should_panic]
    fn task_new_panics_if_empty_weekday_vec() {
        Task::new("Panic!".to_string(), vec![]);
    }

    #[test]
    fn task_remove_weekday_removes_all_duplicates() {
        let mut task = Task::new("Test task".to_string(), vec![Weekday::Mon, Weekday::Tue, Weekday::Wed, Weekday::Wed]);

        task.remove_weekday(Weekday::Wed);

        assert!(task.weekdays().contains(&Weekday::Mon));
        assert!(task.weekdays().contains(&Weekday::Tue));
        assert!(!task.weekdays().contains(&Weekday::Wed));
    }

    #[test]
    fn task_displays_correctly() {
        let task = Task::new("Task".to_string(), vec![Weekday::Wed]);
        assert_eq!(task.to_string(), "Task (ID: 0)".to_string());
    }

    #[test]
    fn tdlist_add_todo_updates_ids() {
        let mut list = TdList::new_client();

        list.add_todo(Todo::new_undated("Todo 0".to_string()));
        list.add_todo(Todo::new_undated("Todo 1".to_string()));
        list.add_todo(Todo::new_undated("Todo 2".to_string()));

        assert_eq!(list.todos()[0].id(), 0);
        assert_eq!(list.todos()[1].id(), 1);
        assert_eq!(list.todos()[2].id(), 2);
    }

    #[test]
    fn tdlist_removed_todos_not_visible() {
        let mut list = TdList::new_client();

        list.add_todo(Todo::new_undated("Todo 0".to_string()));
        list.add_todo(Todo::new_undated("Todo 1".to_string()));
        list.add_todo(Todo::new_undated("Todo 2".to_string()));

        list.remove_todo(1).unwrap();

        assert_eq!(list.todos()[0].body(), "Todo 0");
        assert_eq!(list.todos()[1].body(), "Todo 2");
        assert_eq!(list.todos().len(), 2);
    }

    #[test]
    fn tdlist_remove_todo_returns_err_nonexistent_id() {
        let mut list = TdList::new_client();

        list.add_todo(Todo::new_undated("Todo 0".to_string()));
        list.add_todo(Todo::new_undated("Todo 1".to_string()));

        assert!(list.remove_todo(2).is_err());
    }

    #[test]
    fn tdlist_add_task_updates_ids() {
        let mut list = TdList::new_client();

        list.add_task(Task::new("Task 0".to_string(), vec![Weekday::Mon]));
        list.add_task(Task::new("Task 1".to_string(), vec![Weekday::Mon]));
        list.add_task(Task::new("Task 2".to_string(), vec![Weekday::Mon]));

        assert_eq!(list.tasks()[0].id(), 0);
        assert_eq!(list.tasks()[1].id(), 1);
        assert_eq!(list.tasks()[2].id(), 2);
    }

    #[test]
    fn tdlist_removed_tasks_not_visible() {
        let mut list = TdList::new_client();

        list.add_task(Task::new("Task 0".to_string(), vec![Weekday::Mon]));
        list.add_task(Task::new("Task 1".to_string(), vec![Weekday::Mon]));
        list.add_task(Task::new("Task 2".to_string(), vec![Weekday::Mon]));

        list.remove_task(1).unwrap();

        assert_eq!(list.tasks()[0].body(), "Task 0");
        assert_eq!(list.tasks()[1].body(), "Task 2");
        assert_eq!(list.tasks().len(), 2);
    }

    #[test]
    fn tdlist_remove_task_returns_err_with_nonexistent_id() {
        let mut list = TdList::new_client();

        list.add_task(Task::new("Task 0".to_string(), vec![Weekday::Mon]));
        list.add_task(Task::new("Task 1".to_string(), vec![Weekday::Mon]));

        assert!(list.remove_todo(2).is_err());
    }

    fn tdlist_with_done_and_undone() -> TdList {
        let mut list = TdList::new_client();

        list.add_todo(Todo::new_specific_date("Undone 1".to_string(), NaiveDate::from_ymd(2021, 4, 1)));
        list.add_todo(Todo::new_specific_date("Undone 2".to_string(), NaiveDate::from_ymd(2021, 3, 29)));
        list.add_todo(Todo::new_specific_date("Done 1".to_string(), NaiveDate::from_ymd(2021, 4, 1)));
        list.add_todo(Todo::new_specific_date("Done 2".to_string(), NaiveDate::from_ymd(2021, 3, 30)));

        list.get_todo_mut(2).unwrap().set_done_wtd(true, NaiveDate::from_ymd(2021, 4, 1));
        list.get_todo_mut(3).unwrap().set_done_wtd(true, NaiveDate::from_ymd(2021, 4, 1));

        list.add_task(Task::new("Undone 1".to_string(), vec![Weekday::Thu]));
        list.add_task(Task::new("Done 1".to_string(), vec![Weekday::Thu]));

        list.get_task_mut(1).unwrap().set_done(true, NaiveDate::from_ymd(2021, 4, 1));

        list
    }

    #[test]
    fn tdlist_undone_todos_for_date_returns_only_undone() {
        let list = tdlist_with_done_and_undone();

        let returned = list.undone_todos_for_date_wtd(NaiveDate::from_ymd(2021, 4, 1), NaiveDate::from_ymd(2021, 4, 1));

        assert!(returned.contains(&&list.todos()[0]));
        assert!(returned.contains(&&list.todos()[1]));
        assert!(!returned.contains(&&list.todos()[2]));
        assert!(!returned.contains(&&list.todos()[3]));
        assert_eq!(returned.len(), 2);
    }

    #[test]
    fn tdlist_done_todos_for_date_returns_only_done() {
        let list = tdlist_with_done_and_undone();

        let returned = list.done_todos_for_date_wtd(NaiveDate::from_ymd(2021, 4, 1), NaiveDate::from_ymd(2021, 4, 1));

        assert!(!returned.contains(&&list.todos()[0]));
        assert!(!returned.contains(&&list.todos()[1]));
        assert!(returned.contains(&&list.todos()[2]));
        assert!(returned.contains(&&list.todos()[3]));
        assert_eq!(returned.len(), 2);
    }

    #[test]
    fn tdlist_undone_tasks_for_date_returns_only_undone() {
        let list = tdlist_with_done_and_undone();

        let returned = list.undone_tasks_for_date(NaiveDate::from_ymd(2021, 4, 1));

        assert!(returned.contains(&&list.tasks()[0]));
        assert!(!returned.contains(&&list.tasks()[1]));
        assert_eq!(returned.len(), 1);
    }

    #[test]
    fn tdlist_done_tasks_for_date_returns_only_done() {
        let list = tdlist_with_done_and_undone();

        let returned = list.done_tasks_for_date(NaiveDate::from_ymd(2021, 4, 1));

        assert!(!returned.contains(&&list.tasks()[0]));
        assert!(returned.contains(&&list.tasks()[1]));
        assert_eq!(returned.len(), 1);
    }

    #[test]
    fn tdlist_remove_old_todos_removes_done_after_1_day() {
        let mut list = tdlist_with_done_and_undone();
        let list_containing_same_todos_for_eq_check = tdlist_with_done_and_undone();

        list.remove_old_todos_wtd(NaiveDate::from_ymd(2021, 4, 1));

        assert_eq!(list.todos(), list_containing_same_todos_for_eq_check.todos());

        list.remove_old_todos_wtd(NaiveDate::from_ymd(2021, 4, 2));

        assert_eq!(list.todos()[0], list_containing_same_todos_for_eq_check.todos()[0]);
        assert_eq!(list.todos()[1], list_containing_same_todos_for_eq_check.todos()[1]);
        assert_eq!(list.todos().len(), 2);
    }

    #[test]
    fn tdlist_client_only_self_sync_actually_removes_items() {
        let mut list = tdlist_with_done_and_undone();

        list.remove_old_todos_wtd(NaiveDate::from_ymd(2021, 4, 2));
        list.remove_task(1).unwrap();

        assert_eq!(list.todos.items.len(), 4);
        assert_eq!(list.tasks.items.len(), 2);

        list.self_sync();

        assert_eq!(list.todos.items.len(), 2);
        assert_eq!(list.tasks.items.len(), 1);
    }

    #[test]
    fn tdlist_server_always_removes_items() {
        let mut list = tdlist_with_done_and_undone();
        list.server = true;
        list.todos.server = true;
        list.tasks.server = true;

        list.remove_old_todos_wtd(NaiveDate::from_ymd(2021, 4, 2));
        list.remove_task(1).unwrap();

        assert_eq!(list.todos.items.len(), 2);
        assert_eq!(list.tasks.items.len(), 1);
    }

    #[test]
    fn tdlist_self_sync_always_removes_old_todos() {
        let mut list = tdlist_with_done_and_undone();

        assert_eq!(list.todos.items.len(), 4);

        list.self_sync();

        assert_eq!(list.todos.items.len(), 2);
    }

    #[test]
    fn tdlist_sync_always_removes_old_todos() {
        let mut client = tdlist_with_done_and_undone();
        let mut server = TdList::new_server();

        assert_eq!(client.todos.items.len(), 4);

        client.sync(&mut server);

        assert_eq!(client.todos.items.len(), 2);
    }

    #[test]
    fn tdlist_sync_removed_from_server_gets_removed_from_client() {
        let mut client = TdList::new_client();
        let mut server = TdList::new_server();

        client.add_todo(Todo::new_undated("Todo 1".to_string()));

        client.sync(&mut server);

        server.remove_todo(0).unwrap();

        client.sync(&mut server);

        assert_eq!(client.todos().len(), 0);
        assert_eq!(server.todos().len(), 0);
    }

    #[test]
    fn tdlist_sync_modified_in_server_gets_modified_in_client() {
        let mut client = TdList::new_client();
        let mut server = TdList::new_server();

        client.add_todo(Todo::new_undated("Todo 1".to_string()));

        client.sync(&mut server);

        server.get_todo_mut(0).unwrap().set_body("New Todo 1".to_string());

        client.sync(&mut server);


        assert_eq!(client.todos().len(), 1);
        assert!(client.todos().contains(&&Todo::new_undated("New Todo 1".to_string())));

        assert_eq!(server.todos().len(), 1);
        assert!(server.todos().contains(&&Todo::new_undated("New Todo 1".to_string())));
    }

    #[test]
    fn tdlist_sync_modified_new_gets_copied_to_server() {
        let mut client = TdList::new_client();
        let mut server = TdList::new_server();

        client.add_todo(Todo::new_undated("Todo 1".to_string()));

        client.get_todo_mut(0).unwrap().set_body("New Todo 1".to_string());

        client.sync(&mut server);

        assert_eq!(client.todos().len(), 1);
        assert!(client.todos().contains(&&Todo::new_undated("New Todo 1".to_string())));

        assert_eq!(server.todos().len(), 1);
        assert!(server.todos().contains(&&Todo::new_undated("New Todo 1".to_string())));
    }

    #[test]
    #[should_panic]
    fn tdlist_sync_panics_with_both_server() {
        let mut s = TdList::new_server();
        let mut s1 = TdList::new_server();

        s.sync(&mut s1);
    }

    #[test]
    #[should_panic]
    fn tdlist_sync_panics_with_both_client() {
        let mut s = TdList::new_client();
        let mut s1 = TdList::new_client();

        s.sync(&mut s1);
    }

    // This is like many tests merged into a one due to my laziness.
    #[test]
    fn tdlist_sync_works_with_multiple_items_and_with_tasks() {
        // Overall test: Check that sync works with Tasks.
        let mut client = TdList::new_client();
        let mut server = TdList::new_server();

        // Test 1. Adding multiple works
        client.add_task(Task::new("Task 1".to_string(), vec![Weekday::Fri]));
        client.add_task(Task::new("Task 2".to_string(), vec![Weekday::Fri]));
        client.add_task(Task::new("Task 3".to_string(), vec![Weekday::Fri]));

        server.sync(&mut client);

        assert!(client.tasks().contains(&&Task::new("Task 1".to_string(), vec![Weekday::Fri])));
        assert!(client.tasks().contains(&&Task::new("Task 2".to_string(), vec![Weekday::Fri])));
        assert!(client.tasks().contains(&&Task::new("Task 3".to_string(), vec![Weekday::Fri])));
        assert_eq!(client.tasks().len(), 3);

        assert!(server.tasks().contains(&&Task::new("Task 1".to_string(), vec![Weekday::Fri])));
        assert!(server.tasks().contains(&&Task::new("Task 2".to_string(), vec![Weekday::Fri])));
        assert!(server.tasks().contains(&&Task::new("Task 3".to_string(), vec![Weekday::Fri])));
        assert_eq!(server.tasks().len(), 3);

        // Test 2. Modifying multiple works
        server.tasks.get_item_mut(0).unwrap().set_body("New Task 1".to_string());
        server.tasks.get_item_mut(1).unwrap().set_body("New Task 2".to_string());

        client.sync(&mut server);

        assert!(client.tasks().contains(&&Task::new("New Task 1".to_string(), vec![Weekday::Fri])));
        assert!(client.tasks().contains(&&Task::new("New Task 2".to_string(), vec![Weekday::Fri])));
        assert!(client.tasks().contains(&&Task::new("Task 3".to_string(), vec![Weekday::Fri])));
        assert_eq!(client.tasks().len(), 3);

        assert!(server.tasks().contains(&&Task::new("New Task 1".to_string(), vec![Weekday::Fri])));
        assert!(server.tasks().contains(&&Task::new("New Task 2".to_string(), vec![Weekday::Fri])));
        assert!(server.tasks().contains(&&Task::new("Task 3".to_string(), vec![Weekday::Fri])));
        assert_eq!(server.tasks().len(), 3);

        // Test 3. Removing multiple works.
        client.remove_task(1).unwrap();
        client.remove_task(2).unwrap();

        server.sync(&mut client);

        assert!(client.tasks().contains(&&Task::new("New Task 1".to_string(), vec![Weekday::Fri])));
        assert_eq!(client.tasks().len(), 1);

        assert!(server.tasks().contains(&&Task::new("New Task 1".to_string(), vec![Weekday::Fri])));
        assert_eq!(server.tasks().len(), 1);
    }

    #[test]
    fn tdlist_to_and_from_json_returns_same() {
        let list = tdlist_with_done_and_undone();

        let json = list.to_json().unwrap();

        let list_from_json = TdList::new_from_json(&json).unwrap();

        assert_eq!(list.server, list_from_json.server);
        assert_eq!(list.todos.items, list_from_json.todos.items);
        assert_eq!(list.tasks.items, list_from_json.tasks.items);
        assert_eq!(list.tasks.server, list_from_json.tasks.server);
        assert_eq!(list.todos.server, list_from_json.todos.server);
    }
}