use std::collections::HashMap;
use std::collections::HashSet;
use std::fs;
use std::fs::File;
use std::io::Write;
use std::path::Path;

use argparse::ArgumentParser;
use argparse::Store;
use argparse::StoreTrue;
use quick_xml::events::Event;
use quick_xml::reader::Reader;

#[derive(PartialEq)]
struct LatLng {
    lat: f64,
    lng: f64,
}

struct CoordsFile {
    name: String,
    coords: Vec<LatLng>,
}

fn round_val(value: f64, digits: u32) -> f64 {
    let y = 10i64.pow(digits) as f64;
    (value * y).round() / y
}

fn in_line(a: &LatLng, b: &LatLng, c: &LatLng) -> bool {
    (a.lat - c.lat) * (c.lng - b.lng) == (c.lat - b.lat) * (a.lng - c.lng)
}

struct Options {
    verbose: bool,
    output_path_str: String,
    gpx_path_str: String,
}

fn parse_args() -> Options {
    let mut options = Options {
        verbose: false,
        output_path_str: "".to_string(),
        gpx_path_str: "".to_string(),
    };

    {
        let mut ap = ArgumentParser::new();
        ap.refer(&mut options.verbose)
            .add_option(&["-v", "--verbose"], StoreTrue, "Be verbose");
        ap.refer(&mut options.gpx_path_str)
            .add_option(
                &["-i", "--input-directory"],
                Store,
                "Input directory containing *.gpx files",
            )
            .required();
        ap.refer(&mut options.output_path_str)
            .add_option(
                &["-o", "--output-directory"],
                Store,
                "Output directory containing for the *.js files",
            )
            .required();
        ap.parse_args_or_exit();
    }

    options
}

fn read_files(options: &Options) -> Vec<CoordsFile> {
    let input_path = Path::new(&options.gpx_path_str);

    let paths = fs::read_dir(input_path).unwrap();

    let mut parsed_files: Vec<CoordsFile> = Vec::new();

    let mut buf = Vec::new();
    for path in paths {
        let fullpath = path.unwrap().path().display().to_string();
        if options.verbose {
            println!("Reading: {}", fullpath);
        }

        if !fullpath.ends_with(".gpx") {
            if options.verbose {
                println!("Skipping: {}", fullpath);
            }
            continue;
        }

        let mut coord_file = CoordsFile {
            name: fullpath.clone(),
            coords: vec![],
        };

        let reader_from_file = Reader::from_file(fullpath);
        // TODO handle this properly
        let mut reader = reader_from_file.unwrap();
        reader.config_mut().trim_text(true);

        loop {
            match reader.read_event_into(&mut buf) {
                Err(e) => panic!("Error at position {}: {:?}", reader.buffer_position(), e),
                // exits the loop when reaching end of file
                Ok(Event::Eof) => break,

                Ok(Event::Start(e)) => {
                    if e.name().as_ref() == b"trkpt" {
                        let mut lat: f64 = 0.0;
                        let mut lng: f64 = 0.0;

                        for attr_result in e.attributes() {
                            // TODO handle this properly
                            let a = attr_result.unwrap();

                            match a.key.as_ref() {
                                b"lat" => {
                                    lat = (std::str::from_utf8(&a.value))
                                        .unwrap()
                                        .parse::<f64>()
                                        .unwrap()
                                }
                                b"lon" => {
                                    lng = (std::str::from_utf8(&a.value))
                                        .unwrap()
                                        .parse::<f64>()
                                        .unwrap()
                                }
                                _ => (),
                                }
                        }

                        if options.verbose {
                            println!("Found point {} {}", lat, lng);
                        }

                        if lat != 0.0 && lng != 0.0 {
                            coord_file.coords.push(LatLng { lat, lng });
                        } else if options.verbose {
                            println!("Skipping invalid trpkt");
                        }
                    }
                }
                _ => (),
            }
            buf.clear()
        }
        parsed_files.push(coord_file);
    }

    parsed_files
}

fn round_values(parsed_files: &mut Vec<CoordsFile>, options: &Options) {
    // Round values, example: 51.329793, 6 digits
    for file in parsed_files {
        for coord in &mut file.coords {
            if options.verbose {
                println!("Before {} {}", coord.lat, coord.lng);
            }

            coord.lat = round_val(coord.lat, 6);
            coord.lng = round_val(coord.lng, 6);

            if options.verbose {
                println!("After {} {}", coord.lat, coord.lng);
            }
        }
    }
}

fn remove_duplicates(parsed_files: &mut Vec<CoordsFile>, options: &Options) {
    // Remove duplicates
    for file in parsed_files {
        if options.verbose {
            println!("Before dedup {}", file.coords.len());
        }
        file.coords.dedup();
        if options.verbose {
            println!("After dedup {}", file.coords.len());
        }
    }
}

fn remove_files_without_new_points(parsed_files: &mut Vec<CoordsFile>, options: &Options) {
    // Filter files without any new points
    let mut map: HashMap<String, HashSet<String>> = HashMap::new();
    let mut remove_files: Vec<String> = Vec::new();
    parsed_files.iter_mut().for_each(|file| {
        let mut new_points = false;
        for coord in &file.coords {
            let lat = round_val(coord.lat, 4).to_string();
            let lng = round_val(coord.lng, 4).to_string();

            if map.contains_key(&lat) {
                let hash_coords = map.get_mut(&lat).unwrap();
                if hash_coords.contains(&lng) {
                    continue;
                } else {
                    hash_coords.insert(lng);
                    new_points = true;
                }
            } else {
                let mut new_coords = HashSet::new();
                new_coords.insert(lng);
                map.insert(lat, new_coords);
                new_points = true;
            }
        }

        if !new_points {
            remove_files.push(file.name.clone());
        }
    });

    if options.verbose {
        println!(
            "Files to remove: {}, out of {}",
            remove_files.len(),
            parsed_files.len()
        );
    }
    for remove_file in remove_files {
        let position = parsed_files
            .iter()
            .position(|value| *value.name == remove_file)
            .unwrap();
        parsed_files.remove(position);
    }
    if options.verbose {
        println!("Remaining files: {}", parsed_files.len());
    }
}

fn remove_straight_line_points(parsed_files: &mut Vec<CoordsFile>, options: &Options) {
    // Remove points on the same line
    for file in parsed_files {
        let mut removed_points = 0;
        let coords = &mut file.coords;
        let old_coords = coords.len();
        for i in 0..=coords.len() - 3 {
            // This can happen because we already removed items
            if i + 2 >= coords.len() {
                break;
            }

            if in_line(&coords[i], &coords[i + 2], &coords[i + 1]) {
                if options.verbose {
                    println!(
                        "Removing coord: {} {}, which is between {} {} and {} {}",
                        coords[i + 1].lat,
                        coords[i + 1].lng,
                        coords[i].lat,
                        coords[i].lng,
                        coords[i + 2].lat,
                        coords[i + 2].lng
                    );
                }
                coords.remove(i + 1);
                removed_points += 1;
            }
        }

        if options.verbose {
            println!(
                "Removed points: {} out of {}, from {}",
                removed_points, old_coords, file.name
            );
        }
    }
}

fn output_result_files(parsed_files: &Vec<CoordsFile>, options: &Options) {
    // Final step: write new files
    fs::create_dir_all(&options.output_path_str).unwrap();

    for file in parsed_files {
        let base_path = Path::new(&file.name);
        let filename = base_path.file_name().unwrap();
        let filename_str = filename.to_str().unwrap().replace(".gpx", ".js");

        let out_path = Path::new(&options.output_path_str);
        let file_out_path = out_path.join(filename_str);

        if options.verbose {
            println!("Creating new file: {}", file_out_path.to_str().unwrap());
        }

        let mut out_file = File::create(file_out_path).unwrap();
        let var_name = filename.to_str().unwrap().replace(".gpx", "");
        out_file.write(b"var ").unwrap();
        out_file.write(var_name.as_bytes()).unwrap();
        out_file.write(b" = [").unwrap();
        for coord in &file.coords {
            let mut coord_str =
                String::from("[") + &coord.lat.to_string() + "," + &coord.lng.to_string() + "]";
            if coord != file.coords.last().unwrap() {
                coord_str += ",";
            }
            out_file.write(coord_str.as_bytes()).unwrap();
        }
        out_file.write(b"];").unwrap();
    }
}

fn main() {
    let options = parse_args();

    if options.verbose {
        println!("Input directory: {}", options.gpx_path_str);
        println!("Output directory: {}", options.output_path_str);
    }
    println!("Reading files...");
    let mut parsed_files = read_files(&options);

    // Test
    println!("Parsed files: {}", parsed_files.len());

    let mut sum = 0;
    for file in &parsed_files {
        sum += file.coords.len();
    }
    println!("Parsed points: {}", sum);

    println!("Rounding values...");
    round_values(&mut parsed_files, &options);

    println!("Removing duplicates in file...");
    remove_duplicates(&mut parsed_files, &options);

    println!("Removing tracks without new points...");
    remove_files_without_new_points(&mut parsed_files, &options);

    println!("Removing points on a straight line...");
    remove_straight_line_points(&mut parsed_files, &options);

    println!("Final files: {}", parsed_files.len());
    let mut sum = 0;
    for file in &parsed_files {
        sum += file.coords.len();
    }
    println!("Final points: {}", sum);

    output_result_files(&parsed_files, &options);
}
