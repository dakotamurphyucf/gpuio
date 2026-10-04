//! First displayed month is separate from the calendar's cursor month.
use gpuio_protocol::calendar::Month;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) struct Viewport {
    first: Month,
    count: i64,
}
impl Viewport {
    pub(super) fn new(cursor: Month, count: i64) -> Self {
        let mut this = Self {
            first: cursor,
            count,
        };
        this.configure(count, cursor);
        this
    }
    pub(super) fn first(self) -> Month {
        self.first
    }
    pub(super) fn contains(self, month: Month) -> bool {
        month.index() >= self.first.index() && month.index() < self.first.index() + self.count
    }
    pub(super) fn months(self) -> impl Iterator<Item = Month> {
        (0..self.count).map(move |offset| self.first.shift(offset).expect("bounded viewport"))
    }
    pub(super) fn configure(&mut self, count: i64, cursor: Month) {
        assert!((1..=12).contains(&count), "admitted calendar month count");
        self.count = count;
        let first = self
            .first
            .index()
            .min(cursor.index())
            .max(cursor.index() - count + 1);
        self.set_first_index(first);
    }
    pub(super) fn show(&mut self, month: Month) {
        self.set_first_index(month.index());
    }
    fn set_first_index(&mut self, first: i64) {
        let first = first.clamp(0, 119_988 - self.count);
        self.first = self
            .first
            .shift(first - self.first.index())
            .expect("bounded first month");
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn month(year: i64, month: i64) -> Month {
        Month::new(year, month).unwrap()
    }

    #[test]
    fn cursor_crosses_visible_months_without_shifting_the_view() {
        let feb = month(2024, 2);
        let mut view = Viewport::new(feb, 3);
        for cursor in [month(2024, 3), month(2024, 4), feb] {
            view.configure(3, cursor);
            assert_eq!(view.first(), feb);
            assert!(view.contains(cursor));
        }
        view.configure(3, month(2024, 5));
        assert_eq!(view.first(), month(2024, 3));
        view.configure(3, month(2024, 1));
        assert_eq!(view.first(), month(2024, 1));
    }

    #[test]
    fn count_changes_and_explicit_navigation_keep_cursor_and_civil_bounds() {
        for count in 1..=12 {
            for cursor in [month(1, 1), month(1, 2), month(2024, 2), month(9999, 12)] {
                let mut view = Viewport::new(cursor, count);
                assert!(view.contains(cursor));
                assert_eq!(view.months().count(), count as usize);
                for next in 1..=12 {
                    view.configure(next, cursor);
                    assert!(view.contains(cursor));
                    assert_eq!(view.months().count(), next as usize);
                }
                view.show(month(9999, 12));
                assert_eq!(view.months().last(), Some(month(9999, 12)));
                view.show(month(1, 1));
                assert_eq!(view.first(), month(1, 1));
            }
        }
    }
}
