/// Store how many times an event has happened. [`N`] is the size of a
/// contiguous array.
pub struct Histogram<const N: usize> {
    content: [usize; N],
    content_overflow: Vec<usize>,
}

impl<const N: usize> Default for Histogram<N> {
    fn default() -> Self {
        Self::new()
    }
}

impl<const N: usize> Histogram<N> {
    pub fn new() -> Self {
        Self {
            content: [0; N],
            content_overflow: Vec::with_capacity(64),
        }
    }

    /// Copies all data from [`other`].
    pub fn add_data_from(&mut self, other: &Self) {
        // By definition, both will have the same length.
        for i in 0..N {
            self.content[i] += other.content[i];
        }
        self.content_overflow
            .append(&mut other.content_overflow.clone());
    }

    /// Adds the value to the histogram.
    pub fn add_value(&mut self, value: usize) {
        if value >= N {
            self.content_overflow.push(value);
            return;
        }
        self.content[value] += 1;
    }

    // Gets elements and their corresponding frequency
    fn get_frequency_table_from_overflow(&mut self) -> Vec<(usize, usize)> {
        let mut res: Vec<(usize, usize)> = Vec::new();

        if self.content_overflow.is_empty() {
            return res;
        }

        self.content_overflow.sort();

        let mut latency_count: usize = 0;

        let mut overflow_iter = self.content_overflow.iter().peekable();

        while let Some(latency) = overflow_iter.next() {
            latency_count += 1;

            if let Some(next_latency) = overflow_iter.peek() {
                if latency != *next_latency {
                    res.push((*latency, latency_count));
                    latency_count = 0;
                }
            } else {
                res.push((*latency, latency_count));
            }
        }

        res
    }

    /// Prints all elements. [`remove_empty`] decides if empty elements will be
    /// removed or printed.
    pub fn print(&mut self, remove_empty: bool) {
        let overflow = self.get_frequency_table_from_overflow();

        if remove_empty {
            self.content
                .iter()
                .enumerate()
                .filter(|(_, v)| **v != 0) // remove empty
                .for_each(|(i, v)| println!("{i}\t{v}"));

            overflow.iter().for_each(|(i, v)| println!("{i}\t{v}"));
        } else {
            self.content.iter().for_each(|v| println!("{v}"));

            let mut latency = N;
            for record in overflow.iter() {
                // print zeroes for all missing records
                while latency < record.0 {
                    println!("0");
                    latency += 1;
                }
                println!("{}", record.1);
                latency += 1;
            }
        }
    }

    /// The largest value in the histogram.
    pub fn max_value(&self) -> usize {
        if let Some(max_value) = self.content_overflow.iter().max() {
            return *max_value;
        }
        for (i, val) in self.content.iter().enumerate().rev() {
            if *val != 0 {
                return i;
            }
        }
        0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_add_value() {
        let mut h: Histogram<100> = Histogram::new();
        h.add_value(98);
        h.add_value(99);
        h.add_value(101);
        h.add_value(100);
    }

    #[test]
    fn test_overflow() {
        let mut h: Histogram<5> = Histogram::new();
        h.add_value(0);
        h.add_value(4);
        h.add_value(5);
        h.add_value(6);
        h.add_value(6);
        h.add_value(9);

        assert!(h.content_overflow.len() == 4);

        assert_eq!(
            h.get_frequency_table_from_overflow(),
            vec![(5, 1), (6, 2), (9, 1)]
        )
    }
}
