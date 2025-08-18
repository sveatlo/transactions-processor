use chrono::{DateTime, Local};

fn main() {
    set_build_timestamp();
}

fn set_build_timestamp() {
    let build_datetime = option_env!("GIT_TIMESTAMP").map_or_else(Local::now, |timestamp_secs| {
        let timestamp_secs = timestamp_secs
            .parse::<i64>()
            .expect("BUG: failed to parse timestamp_secs");

        let utc = DateTime::from_timestamp(timestamp_secs, 0)
            .expect("BUG: failed to create NaiveDateTime from timestamp_secs");

        DateTime::<Local>::from(utc)
    });

    let build_datetime = build_datetime.to_rfc3339();

    println!("cargo:rustc-env=GIT_TIMESTAMP={build_datetime}");
}
