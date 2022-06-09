use chrono::{Date, Datelike, Local, Weekday};

/// Gets the date that represents the upcoming weekday. Given tomorrow’s weekday, this should return
/// tomorrows date. Today is represented by the current weekday.
fn weekday_to_date(weekday: Weekday, mut today: Date<Local>) -> Date<Local> {
    loop {
        if today.weekday() == weekday {
            return today;
        }
        today = today.succ();
    }
}

/// Represents a one-time task to be done at a specific weekday.
pub struct Todo {
    body: String,
    date: Date<Local>,
}

impl Todo {
    /// Creates a new `Todo` that doesn't have a weekday associated with it.
    pub fn new_undated(body: String) -> Todo {
        Todo {
            body,
            date: Local::today(),
        }
    }

    /// Creates a new `Todo` that has a weekday associated with it.
    pub fn new_dated(body: String, weekday: Weekday) -> Todo {
        Todo {
            body,
            date: weekday_to_date(weekday, Local::today()),
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::weekday_to_date;

    // Unit test a private function to remove the need to pass today into the Todo constructor
    #[test]
    fn weekday_to_date_returns_correct_dates() {
        use chrono::{Local, TimeZone, Weekday};

        // Today is a Tuesday
        let today = Local.ymd(2022,6,7);

        // Tue should return today’s date
        assert_eq!(weekday_to_date(Weekday::Tue, today), today);

        // Wed should return tomorrow’s date
        assert_eq!(weekday_to_date(Weekday::Wed, today), today.succ());

        // Mon should return next weeks monday
        assert_eq!(weekday_to_date(Weekday::Mon, today), Local.ymd(2022,6,13));
    }

}