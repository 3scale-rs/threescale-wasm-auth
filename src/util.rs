pub fn serde_json_error_lines<'i, 'e: 'i>(
    e: &'e serde_json::Error,
    input: &'i str,
    before_ctx: usize,
    after_ctx: usize,
) -> impl Iterator<Item = String> + 'i {
    let line = e.line();
    let column = e.column();
    let line_skip = line.checked_sub(before_ctx.saturating_add(1)).unwrap_or(0);
    // before_len also takes the error line
    let before_len = line - line_skip;
    let numchars = |mut num: usize| {
        let mut chars: usize = 1;
        while num > 9 {
            num /= 10;
            chars += 1;
        }
        chars
    };
    let last_line = (0..=after_ctx)
        .rev()
        .find_map(|after| line.checked_add(after))
        .unwrap_or(line);
    let after_len = last_line - line;
    let lineno_width = numchars(last_line);
    let format_line = move |(current_line, line)| {
        format!(
            "{:>width$}: {}",
            current_line + line_skip + 1,
            line,
            width = lineno_width
        )
    };
    let before_it = input.lines().skip(line_skip);
    let after_it = before_it
        .clone()
        .enumerate()
        .skip(before_len)
        .take(after_len)
        .map(format_line);
    before_it
        .enumerate()
        .take(before_len)
        .map(format_line)
        .chain(core::iter::once(format!(
            "{: >width$}  {: >columns$} error ({:?}) {}",
            "",
            "^",
            e.classify(),
            e,
            width = lineno_width,
            columns = column
        )))
        .chain(after_it)
}

pub fn serde_json_error_to_string<'i, 'e: 'i>(e: &'e serde_json::Error, input: &'i str) -> String {
    serde_json_error_lines(e, input, 2, 2)
        .collect::<Vec<_>>()
        .join("\n")
}
