use chrono::{Date, Datelike, Local, Weekday};

/// Gets the date that represents the upcoming weekday. Given tomorrow’s weekday, this should return
/// tomorrows date. Today is represented by the current weekday.
/// # Example
/// ```
/// use chrono::{Local, TimeZone, Weekday};
/// use mtd::weekday_to_date;
///
/// // Today is a Tuesday
/// let today = Local.ymd(2022,6,7);
///
/// // Tue should return today’s date
/// assert_eq!(weekday_to_date(Weekday::Tue, today), today);
///
/// // Wed should return tomorrow’s date
/// assert_eq!(weekday_to_date(Weekday::Wed, today), today.succ());
///
/// // Mon should return next weeks monday
/// assert_eq!(weekday_to_date(Weekday::Mon, today), Local.ymd(2022,6,13));
/// ```
pub fn weekday_to_date(weekday: Weekday, mut today: Date<Local>) -> Date<Local> {
    loop {
        if today.weekday() == weekday {
            return today;
        }
        today = today.succ();
    }
}

#[cfg(test)]
mod tests {
}