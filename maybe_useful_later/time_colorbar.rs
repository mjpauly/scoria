// attempt at labeling cmin/cmax, but it's hard.
// can return to doing a proper colorbar for time later
else if **colored_datastream == ColoredDataStream::Time {
    log::debug!("{} {}", cmin, cmax);
    let local_offset =
        time::UtcOffset::current_local_offset().unwrap();
    let tick_text = [cmin, cmax]
        .iter()
        .map(|x| {
            let t = time::OffsetDateTime::from_unix_timestamp(
                *x as i64,
            )
            .unwrap();
            let mut t = t
        .to_offset(local_offset)
        .format(&time::format_description::well_known::Rfc2822)
        .unwrap();
            t.truncate(25);
            t
        })
        .collect();
    log::debug!("{:?}", tick_text);
    colorbar = colorbar
        .tick_vals(vec![cmin, cmax])
        .tick_text(tick_text)
        .x_pad(100.); // override more padding for long labels
}
