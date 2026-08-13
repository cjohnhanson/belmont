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
        let mut entries: Vec<(String, String)> =
            entries.into_iter().filter(|(_, v)| !v.is_empty()).collect();
        entries.sort_by_key(|(_, v)| std::cmp::Reverse(v.len()));
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

        // Cut the buffer at a point no secret spans. Everything before the
        // cut is fully determined: a future chunk cannot change it, because
        // a secret is at most max_len bytes and the cut retains that many.
        // Scrubbing the prefix alone is therefore correct, and the tail
        // stays buffered for the next chunk.
        let cut = self.safe_cut();
        if cut == 0 {
            return String::new();
        }
        let prefix = self.scrub_text(&self.buffer[..cut]);
        self.buffer = self.buffer[cut..].to_string();
        prefix
    }

    /// The largest safe cut point in the buffer, in bytes.
    ///
    /// The start is the last `max_len` bytes rounded down to a char
    /// boundary, so no slice ever falls inside a multibyte character. The
    /// cut then moves left past any secret occurrence that straddles it,
    /// so a secret is never split across the cut.
    fn safe_cut(&self) -> usize {
        let mut cut = self.buffer.len() - self.max_len;
        while !self.buffer.is_char_boundary(cut) {
            cut -= 1;
        }
        loop {
            let mut moved = false;
            for (_, value) in &self.entries {
                if value.is_empty() {
                    continue;
                }
                let mut from = 0;
                while let Some(rel) = self.buffer[from..].find(value.as_str()) {
                    let start = from + rel;
                    let end = start + value.len();
                    if start < cut && end > cut {
                        cut = start;
                        moved = true;
                        break;
                    }
                    from = start + 1;
                }
            }
            if !moved {
                return cut;
            }
        }
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
    fn non_ascii_output_does_not_panic_and_scrubs() {
        // The old byte-slice at max_len fell inside a multibyte char and
        // panicked. Multibyte content around a secret must be safe.
        let mut s = make_scrubber(vec![("K", "hunter2")]);
        let out = s.feed("préfix ααα hunter2 βββ suffix with ünïcode padding");
        let out = format!("{out}{}", s.flush());
        assert!(out.contains("belmont://K"), "secret scrubbed: {out}");
        assert!(
            out.contains("préfix") && out.contains("ünïcode"),
            "text intact: {out}"
        );
        assert!(!out.contains("hunter2"), "secret must not leak: {out}");
    }

    #[test]
    fn secret_straddling_the_cut_is_never_emitted_raw() {
        // Feed byte by byte around the boundary. No emitted output may
        // ever contain the raw secret.
        let mut s = make_scrubber(vec![("K", "topsecret")]);
        let mut emitted = String::new();
        for ch in "leading padding text topsecret trailing padding text".chars() {
            emitted.push_str(&s.feed(&ch.to_string()));
        }
        emitted.push_str(&s.flush());
        assert!(
            !emitted.contains("topsecret"),
            "raw secret leaked: {emitted}"
        );
        assert!(emitted.contains("belmont://K"), "scrubbed: {emitted}");
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
