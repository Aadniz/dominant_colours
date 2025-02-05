#![deny(warnings)]

#[macro_use]
extern crate clap;

use kmeans_colors::get_kmeans_hamerly;
use rand::random;
use palette::{FromColor, IntoColor, Pixel, Lab, Srgb, Srgba};

mod cli;
mod get_bytes;
mod terminal_colours;

fn main() {
    let matches = cli::app().get_matches();

    let path = matches
        .get_one::<String>("PATH")
        .expect("`path` is required");

    let terminal_colours = matches
        .get_flag("terminal-colours");

    let random_seed = matches
        .get_flag("random-seed");

    let max_brightness = matches
        .get_flag("max-brightness");

    let seed: u64 = if random_seed { random() } else {
        *matches
            .get_one::<u64>("SEED")
            .expect("`seed` is required")
    };

    let colour_count = *matches
        .get_one::<usize>("MAX-COLOURS")
        .expect("`max-colours` is required");

    let colour_count: usize = if terminal_colours && 16 > colour_count { 16 } else { colour_count };

    // There's different code for fetching bytes from GIF images because
    // GIFs are often animated, and we want a selection of frames.
    let img_bytes = if path.to_lowercase().ends_with(".gif") {
        get_bytes::get_bytes_for_gif(&path)
    } else {
        get_bytes::get_bytes_for_image(&path)
    };

    // This is based on code from the kmeans-colors binary, but with a bunch of
    // the options stripped out.
    // See https://github.com/okaneco/kmeans-colors/blob/0.5.0/src/bin/kmeans_colors/app.rs
    let lab: Vec<Lab> = Srgba::from_raw_slice(&img_bytes)
        .iter()
        .map(|x| x.into_format::<_, f32>().into_color())
        .collect();

    let max_iterations = 20;
    let converge = 1.0;
    let verbose = false;

    let result = get_kmeans_hamerly(colour_count, max_iterations, converge, verbose, &lab, seed).centroids;

    let srgb_colors = result
        .iter()
        .map(|x| Srgb::from_color(*x).into_format())
        .collect();

    let rgb = if terminal_colours {
        terminal_colours::create_terminal_colour(srgb_colors, max_brightness)
    } else {
        srgb_colors
    };

    // This uses ANSI escape sequences and Unicode block elements to print
    // a palette of hex strings which are coloured to match.
    // See https://alexwlchan.net/2021/04/coloured-squares/
    for c in rgb {
        let display_value = format!("#{:02x}{:02x}{:02x}", c.red, c.green, c.blue);

        if matches.get_flag("no-palette") {
            println!("{}", display_value);
        } else {
            println!(
                "\x1B[38;2;{};{};{}m▇ {}\x1B[0m",
                c.red, c.green, c.blue, display_value
            );
        }
    }
}

#[cfg(test)]
mod tests {
    use std::str;

    use assert_cmd::assert::OutputAssertExt;
    use assert_cmd::Command;

    // Note: for the purposes of these tests, I mostly trust the k-means code
    // provided by the external library.

    #[test]
    fn it_prints_the_colour_with_ansi_escape_codes() {
        let output = get_success(&["./src/tests/red.png", "--max-colours=1"]);

        assert_eq!(output.exit_code, 0);

        assert!(
            output.stdout == "\u{1b}[38;2;255;0;0m▇ #ff0000\u{1b}[0m\n"
                || output.stdout == "\u{1b}[38;2;254;0;0m▇ #fe0000\u{1b}[0m\n",
            "stdout = {:?}",
            output.stdout
        );

        assert_eq!(output.stderr, "");
    }

    #[test]
    fn it_can_look_at_png_images() {
        let output = get_success(&["./src/tests/red.png", "--max-colours=1"]);
        assert_eq!(output.exit_code, 0);
    }

    #[test]
    fn it_can_look_at_jpeg_images() {
        let output = get_success(&["./src/tests/noise.jpg", "--max-colours=1"]);
        assert_eq!(output.exit_code, 0);
    }

    #[test]
    fn it_can_look_at_static_gif_images() {
        let output = get_success(&["./src/tests/yellow.gif", "--max-colours=1"]);
        assert_eq!(output.exit_code, 0);
    }

    #[test]
    fn it_can_look_at_tiff_images() {
        let output = get_success(&["./src/tests/green.tiff", "--max-colours=1"]);
        assert_eq!(output.exit_code, 0);
    }

    #[test]
    fn it_omits_the_escape_codes_with_no_palette() {
        let output = get_success(&["./src/tests/red.png", "--max-colours=1", "--no-palette"]);

        assert_eq!(output.exit_code, 0);

        assert!(
            output.stdout == "#ff0000\n" || output.stdout == "#fe0000\n",
            "stdout = {:?}",
            output.stdout
        );

        assert_eq!(output.stderr, "");
    }

    #[test]
    fn it_defaults_to_five_colours() {
        let output = get_success(&["./src/tests/noise.jpg"]);

        assert_eq!(
            output.stdout.matches("\n").count(),
            5,
            "stdout = {:?}",
            output.stdout
        );
    }

    #[test]
    fn it_lets_you_choose_the_max_colours() {
        let output = get_success(&["./src/tests/noise.jpg", "--max-colours=8"]);

        assert_eq!(
            output.stdout.matches("\n").count(),
            8,
            "stdout = {:?}",
            output.stdout
        );
    }

    #[test]
    fn it_lets_you_choose_the_seed() {
        let output = get_success(&["./src/tests/noise.jpg", "--max-colours=1", "--seed", "123456789"]);

        assert_eq!(output.stdout.contains("#85827f"), true);
    }

    #[test]
    fn it_lets_you_set_random_seed() {
        let output1 = get_success(&["./src/tests/noise.jpg", "--random-seed"]);
        let output2 = get_success(&["./src/tests/noise.jpg", "--random-seed"]);

        assert_ne!(
            output1.stdout,
            output2.stdout
        );
    }

    // The image created in the next two tests was created with the
    // following command:
    //
    //      convert -delay 200 -loop 10 -dispose previous red.png blue.png red.png blue.png red.png blue.png red.png blue.png animated_squares.gif
    //

    #[test]
    fn it_looks_at_multiple_frames_in_an_animated_gif() {
        let output = get_success(&["./src/tests/animated_squares.gif"]);

        assert_eq!(
            output.stdout.matches("\n").count(),
            2,
            "stdout = {:?}",
            output.stdout
        );
    }

    #[test]
    fn it_looks_at_multiple_frames_in_an_animated_gif_uppercase() {
        let output = get_success(&["./src/tests/animated_upper_squares.GIF"]);

        assert_eq!(
            output.stdout.matches("\n").count(),
            2,
            "stdout = {:?}",
            output.stdout
        );
    }

    #[test]
    fn it_still_prints_16_colours_when_max_colours_and_terminal_colours_are_set() {
        let output = get_success(&["./src/tests/terminal_colours.png", "--terminal-colours", "--max-colours=20"]);

        assert_eq!(output.exit_code, 0);

        assert_eq!(
            output.stdout.matches("\n").count(),
            16,
            "stdout = {:?}",
            output.stdout
        );
    }

    // Notice the colours in the terminal_colours.png image is slightly different than the values defined in terminal_colours.rs.
    // This is on purpose to test that slight variation gets handled.
    #[test]
    fn it_prints_the_ansi_terminal_colours_mapped_correctly() {
        let output = get_success(&["./src/tests/terminal_colours.png", "--terminal-colours", "--no-palette"]);

        assert_eq!(output.exit_code, 0);

        let expected_output = "\
#000000
#aa0000
#00aa00
#808000
#0000aa
#aa00aa
#00aaaa
#aaaaaa
#555555
#ff0000
#00ff00
#ffff00
#0000ff
#ff00ff
#00ffff
#ffffff
";

        assert_eq!(output.stdout, expected_output);

        assert_eq!(output.stderr, "");
    }

    #[test]
    fn it_fails_if_you_pass_an_invalid_max_colours() {
        let output = get_failure(&["./src/tests/red.png", "--max-colours=NaN"]);

        assert_eq!(output.exit_code, 2);
        assert_eq!(output.stdout, "");
        assert_eq!(
            output.stderr,
            "error: Invalid value 'NaN' for '--max-colours <MAX-COLOURS>': invalid digit found in string\n\nFor more information try '--help'\n"
        );
    }

    #[test]
    fn it_fails_if_you_pass_an_invalid_seed() {
        let output = get_failure(&["./src/tests/noise.jpg", "--seed=NaN"]);

        assert_eq!(output.exit_code, 2);
        assert_eq!(output.stdout, "");
        assert_eq!(
            output.stderr,
            "error: Invalid value 'NaN' for '--seed <SEED>': invalid digit found in string\n\nFor more information try '--help'\n"
        );
    }

    #[test]
    fn it_fails_if_you_pass_an_nonexistent_file() {
        let output = get_failure(&["./doesnotexist.jpg"]);

        assert_eq!(output.exit_code, 1);
        assert_eq!(output.stdout, "");
        assert_eq!(output.stderr, "No such file or directory (os error 2)\n");
    }

    #[test]
    fn it_fails_if_you_pass_an_nonexistent_gif() {
        let output = get_failure(&["./doesnotexist.gif"]);

        assert_eq!(output.exit_code, 1);
        assert_eq!(output.stdout, "");
        assert_eq!(output.stderr, "No such file or directory (os error 2)\n");
    }

    #[test]
    fn it_fails_if_you_pass_a_non_image_file() {
        let output = get_failure(&["./README.md"]);

        assert_eq!(output.exit_code, 1);
        assert_eq!(output.stdout, "");
        assert_eq!(
            output.stderr,
            "The file extension `.\"md\"` was not recognized as an image format\n"
        );
    }

    #[test]
    fn it_fails_if_you_pass_an_unsupported_image_format() {
        let output = get_failure(&["./src/tests/purple.webp"]);

        assert_eq!(output.exit_code, 1);
        assert_eq!(output.stdout, "");
        assert_eq!(output.stderr, "The image format WebP is not supported\n");
    }

    #[test]
    fn it_fails_if_you_pass_a_malformed_image() {
        let output = get_failure(&["./src/tests/malformed.txt.png"]);

        assert_eq!(output.exit_code, 1);
        assert_eq!(output.stdout, "");
        assert_eq!(
            output.stderr,
            "Format error decoding Png: Invalid PNG signature.\n"
        );
    }

    struct DcOutput {
        exit_code: i32,
        stdout: String,
        stderr: String,
    }

    fn get_success(args: &[&str]) -> DcOutput {
        let mut cmd = Command::cargo_bin("dominant_colours").unwrap();
        let output = cmd
            .args(args)
            .unwrap()
            .assert()
            .success()
            .get_output()
            .to_owned();

        DcOutput {
            exit_code: output.status.code().unwrap(),
            stdout: str::from_utf8(&output.stdout).unwrap().to_owned(),
            stderr: str::from_utf8(&output.stderr).unwrap().to_owned(),
        }
    }

    fn get_failure(args: &[&str]) -> DcOutput {
        let mut cmd = Command::cargo_bin("dominant_colours").unwrap();
        let output = cmd.args(args).unwrap_err().as_output().unwrap().to_owned();

        DcOutput {
            exit_code: output.status.code().unwrap(),
            stdout: str::from_utf8(&output.stdout).unwrap().to_owned(),
            stderr: str::from_utf8(&output.stderr).unwrap().to_owned(),
        }
    }
}
