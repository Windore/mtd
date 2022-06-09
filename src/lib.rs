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

/// Represents a one-time task to be done at a specific date. The date is specified as a weekday
/// from now. If no weekday is given, the current weekday will be used. After the given weekday, the
/// `Todo` will show up for the current day.
pub struct Todo {
    body: String,
    date: Date<Local>,
}

impl Todo {
    /// Creates a new `Todo` that shows up to be done for the current day.
    pub fn new_undated(body: String) -> Todo {
        Todo {
            body,
            date: Local::today(),
        }
    }

    /// Creates a new `Todo` that shows up to be done at a specific weekday after which it will show
    /// for the current day.
    pub fn new_dated(body: String, weekday: Weekday) -> Todo {
        Todo {
            body,
            date: weekday_to_date(weekday, Local::today()),
        }
    }

    /// Gets the `body` of the `Todo`.
    pub fn body(&self) -> &str {
        &self.body
    }

    /// Gets the weekday of the `Todo`.
    pub fn weekday(&self) -> Weekday {
        self.date.weekday()
    }

    /// Sets the `body` of the `Todo`.
    pub fn set_body(&mut self, body: String) {
        self.body = body;
    }

    /// Sets the weekday of the `Todo`.
    pub fn set_weekday(&mut self, weekday: Weekday) {
        self.date = weekday_to_date(weekday, Local::today());
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
        let today = Local.ymd(2022, 6, 7);

        // Tue should return today’s date
        assert_eq!(weekday_to_date(Weekday::Tue, today), today);

        // Wed should return tomorrow’s date
        assert_eq!(weekday_to_date(Weekday::Wed, today), today.succ());

        // Mon should return next weeks monday
        assert_eq!(weekday_to_date(Weekday::Mon, today), Local.ymd(2022, 6, 13));
    }
}