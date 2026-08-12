/// A streaming scrubber. It replaces each secret value with a `belmont://NAME`
/// reference.
///
/// The scrubber keeps a boundary buffer. The buffer holds a secret that spans
/// two chunks. The scrubber sorts the values longest first. A value that is a
/// substring of another value then gets the correct replacement.
pub struct Scrubber {
    /// The secret entries, sorted by value length, longest first.
    entries: Vec<(String, String)>,
    /// The trailing bytes from the previous `feed()` call. They can hold the
    /// start of a secret value that spans a chunk boundary.
    buffer: String,
    /// The length of the longest secret value. It sets the size of the
    /// boundary buffer.
    max_len: usize,
}

impl Scrubber {
    /// Create a scrubber from name and value pairs. The scrubber removes the
    /// empty values. It sorts the values longest first.
    pub fn new(entries: Vec<(String, String)>) -> Self {
        let mut entries: Vec<(String, String)> = entries
            .into_iter()
            .filter(|(_, v)| !v.is_empty())
            .collect();
        entries.sort_by(|a, b| b.1.len().cmp(&a.1.len()));
        let max_len = entries.first().map(|(_, v)| v.len()).unwrap_or(0);
        Scrubber {
            entries,
            buffer: String::new(),
            max_len,
        }
    }

    /// Feed a chunk of output. Return the prefix that is safe to emit. The
    /// scrubber keeps up to `max_len` bytes in the boundary buffer.
    pub fn feed(&mut self, chunk: &str) -> String {
        if self.entries.is_empty() {
            return chunk.to_string();
        }

        self.buffer.push_str(chunk);

        if self.buffer.len() <= self.max_len {
            // There is not enough data. A secret can still span the boundary.
            return String::new();
        }

        // The bytes before the last max_len bytes are safe to emit.
        let emit_len = self.buffer.len() - self.max_len;
        let scrubbed_full = self.scrub_text(&self.buffer);
        let remaining_original = self.buffer[emit_len..].to_string();
        self.buffer = remaining_original;

        // The scrubber must emit the scrubbed prefix. It cannot scrub the
        // prefix and the tail separately, because a secret can span the two.
        // So it scrubs the full buffer and then finds the cut point.
        //
        // The emittable part is everything that maps to the original bytes
        // before emit_len. Scrubbing changes the string length, so the
        // scrubber scrubs the remaining buffer again and subtracts it.
        let scrubbed_remaining = self.scrub_text(&self.buffer);

        // The emitted part is scrubbed_full without scrubbed_remaining at the
        // end. This works because the tail is still in self.buffer.
        if let Some(prefix) = scrubbed_full.strip_suffix(&scrubbed_remaining) {
            return prefix.to_string();
        }

        // Fallback. Overlapping replacements can move the boundaries and stop
        // the strip from working. Then emit the full scrubbed buffer and clear
        // the buffer. This drops the boundary guarantee, but it loses no data.
        self.buffer.clear();
        scrubbed_full
    }

    /// Flush the remaining buffered bytes at EOF.
    pub fn flush(&mut self) -> String {
        let remaining = std::mem::take(&mut self.buffer);
        self.scrub_text(&remaining)
    }

    /// Replace every secret value with its `belmont://NAME` reference.
    fn scrub_text(&self, text: &str) -> String {
        let mut result = text.to_string();
        for (name, value) in &self.entries {
            result = result.replace(value, &format!("belmont://{name}"));
        }
        result
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_scrubber(entries: Vec<(&str, &str)>) -> Scrubber {
        Scrubber::new(
            entries
                .into_iter()
                .map(|(n, v)| (n.to_string(), v.to_string()))
                .collect(),
        )
    }

    #[test]
    fn single_chunk_replaces_secret() {
        let mut s = make_scrubber(vec![("DB", "hunter2")]);
        let out = s.feed("connecting to hunter2 now");
        let out = format!("{out}{}", s.flush());
        assert_eq!(out, "connecting to belmont://DB now");
    }

    #[test]
    fn multiple_occurrences() {
        let mut s = make_scrubber(vec![("KEY", "abc123")]);
        let out = s.feed("first abc123 second abc123 done");
        let out = format!("{out}{}", s.flush());
        assert_eq!(out, "first belmont://KEY second belmont://KEY done");
    }

    #[test]
    fn secret_spanning_chunk_boundary() {
        let mut s = make_scrubber(vec![("SECRET", "boundary")]);
        let out1 = s.feed("before boun");
        let out2 = s.feed("dary after");
        let out3 = s.flush();
        let combined = format!("{out1}{out2}{out3}");
        assert_eq!(combined, "before belmont://SECRET after");
    }

    #[test]
    fn longer_value_replaced_first() {
        let mut s = make_scrubber(vec![("SHORT", "abc"), ("LONG", "abcdef")]);
        let out = s.feed("value is abcdef here");
        let out = format!("{out}{}", s.flush());
        assert_eq!(out, "value is belmont://LONG here");
    }

    #[test]
    fn empty_values_filtered() {
        let s = make_scrubber(vec![("EMPTY", ""), ("REAL", "secret")]);
        assert_eq!(s.entries.len(), 1);
    }

    #[test]
    fn no_secrets_passthrough() {
        let mut s = make_scrubber(vec![]);
        let out = s.feed("hello world");
        assert_eq!(out, "hello world");
    }

    #[test]
    fn flush_emits_remaining() {
        let mut s = make_scrubber(vec![("X", "secret")]);
        let out1 = s.feed("sec");
        // There is not enough data, so the scrubber buffers it
        assert_eq!(out1, "");
        let out2 = s.flush();
        assert_eq!(out2, "sec");
    }

    #[test]
    fn multiple_secrets_in_one_chunk() {
        let mut s = make_scrubber(vec![("A", "alpha"), ("B", "beta")]);
        let out = s.feed("start alpha middle beta end");
        let out = format!("{out}{}", s.flush());
        assert_eq!(out, "start belmont://A middle belmont://B end");
    }

    #[test]
    fn output_containing_belmont_reference_untouched() {
        let mut s = make_scrubber(vec![("X", "secret")]);
        let out = s.feed("already has belmont://X in it");
        let out = format!("{out}{}", s.flush());
        assert_eq!(out, "already has belmont://X in it");
    }

    #[test]
    fn substring_secret_not_replaced_when_longer_matches() {
        // "pass" is a substring of "password". The longer value wins.
        let mut s = make_scrubber(vec![("PARTIAL", "pass"), ("FULL", "password")]);
        let out = s.feed("my password is here");
        let out = format!("{out}{}", s.flush());
        assert_eq!(out, "my belmont://FULL is here");
    }
}
