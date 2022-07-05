use std::{fs, io, process};
use std::borrow::Borrow;
use std::io::ErrorKind;

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
    /// Runs MTD as a server
    Server,
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
    fn new() -> Result<Self> {
        let config_path = dirs::config_dir().ok_or(Error::Other)?.join("/mtd/mtd.conf");
        let conf;

        if config_path.exists() {
            conf = Config::new_from_json(&fs::read_to_string(config_path)?)?;
        } else {
            conf = Config::new_default(Vec::new(), "127.0.0.1:55995".parse().unwrap());
        }

        let list = TdList::new_from_json(
            &fs::read_to_string(
                conf.save_location()
                    .ok_or(io::Error::from(ErrorKind::NotFound))?
            )?
        )?;

        Ok(Self {
            conf,
            list,
        })
    }
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
}