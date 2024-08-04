macro_rules! light_blue_color {
    () => {
        "\x1b[38;5;12m"
    };
}

macro_rules! lavanda_color {
    () => {
        "\x1b[38;5;146m"
    };
}

macro_rules! mayflower_color {
    () => {
        "\x1b[38;5;11m"
    };
}

macro_rules! gray_color {
    () => {
        "\x1b[38;5;8m"
    };
}

macro_rules! default_color {
    () => {
        "\x1b[0m"
    };
}

macro_rules! auth {
    ($e:expr) => {
        concat!(mayflower_color!(), "AUTH", default_color!(), " ", $e)
    };
}

macro_rules! recv {
    ($e:expr) => {
        concat!(
            light_blue_color!(),
            "RECV",
            gray_color!(),
            " ",
            $e,
            default_color!()
        )
    };
}

macro_rules! gen {
    ($e:expr) => {
        concat!(lavanda_color!(), "GEN ", default_color!(), " ", $e)
    };
}
