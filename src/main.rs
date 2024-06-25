use std::collections::HashMap;
use std::collections::HashSet;
use std::env;
use std::fs;
use std::fs::File;
use std::io::BufReader;
use std::io::Write;
use std::path::Path;
use std::process::exit;

use xml::reader::{EventReader, XmlEvent};

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
    return (value * y).round() / y;
}

fn in_line(a: &LatLng, b: &LatLng, c: &LatLng) -> bool {
    return (a.lat - c.lat) * (c.lng - b.lng) == (c.lat - b.lat) * (a.lng - c.lng);
}

fn main() {
    let args: Vec<String> = env::args().collect();

    if args.len() < 3 {
        exit(1)
    }

    let gpx_path_str = &args[1];
    let output_path_str = &args[2];

    let paths = fs::read_dir(gpx_path_str).unwrap();

    let mut parsed_files: Vec<CoordsFile> = Vec::with_capacity(0);

    for path in paths {
        let fullpath = path.unwrap().path().display().to_string();
        println!("Reading: {}", fullpath);

        if !fullpath.ends_with(".gpx") {
            println!("Skipping: {}", fullpath);
            continue;
        }

        let mut coord_file = CoordsFile {
            name: fullpath.clone(),
            coords: vec![],
        };
        //cFile.name = fullpath.clone();

        let file = File::open(fullpath).unwrap();
        let file = BufReader::new(file);

        let parser = EventReader::new(file);
        for e in parser {
            match e {
                Ok(XmlEvent::StartElement {
                    name, attributes, ..
                }) => {
                    if name.local_name == "trkpt" {
                        let mut lat: f64 = 0.0;
                        let mut lng: f64 = 0.0;
                        for attr in attributes {
                            if attr.name.local_name == "lat" {
                                lat = attr.value.parse::<f64>().unwrap();
                                //println!("Found point: {} {}", attr.name, attr.value);
                            }
                            if attr.name.local_name == "lon" {
                                lng = attr.value.parse::<f64>().unwrap();
                                //println!("Found point: {} {}", attr.name, attr.value);
                            }
                        }

                        if lat != 0.0 || lng != 0.0 {
                            coord_file.coords.push(LatLng { lat: lat, lng: lng });
                        } else {
                            println!("Skipping invalid trpkt");
                        }
                    }
                }
                Err(e) => {
                    eprintln!("Error: {e}");
                    break;
                }
                // There's more: https://docs.rs/xml-rs/latest/xml/reader/enum.XmlEvent.html
                _ => {}
            }
        }

        parsed_files.push(coord_file);
    }

    // Test
    println!("Parsed files: {}", parsed_files.len());
    let mut sum = 0;
    for file in &parsed_files {
        sum += file.coords.len();
    }
    println!("Parsed points: {}", sum);

    // Round values, example: 51.329793, 6 digits

    println!("Rounding values");
    for file in &mut parsed_files {
        for coord in &mut file.coords {
            //println!("Before {} {}", coord.lat, coord.lng);

            coord.lat = round_val(coord.lat, 6);
            coord.lng = round_val(coord.lng, 6);
            //println!("After {} {}", coord.lat, coord.lng);
        }
    }

    // Remove duplicates
    println!("Removing duplicates in file");
    for file in &mut parsed_files {
        //println!("Before dedup {}", file.coords.len());
        file.coords.dedup();
        //println!("After dedup {}", file.coords.len());
    }

    // Filter files without any new points
    println!("Removing tracks without new points");
    let mut map: HashMap<String, HashSet<String>> = HashMap::new();
    let mut remove_files: Vec<String> = Vec::new();
    for file in &parsed_files {
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

        if false == new_points {
            remove_files.push(file.name.clone());
        }
    }

    // println!(
    //    "Files to remove: {}, out of {}",
    //    remove_files.len(),
    //    parsed_files.len()
    //);
    for remove_file in remove_files {
        let position = parsed_files
            .iter()
            .position(|value| *value.name == remove_file)
            .unwrap();
        parsed_files.remove(position);
    }
    //println!("Remaining files: {}", parsed_files.len());

    // Remove points on the same line
    println!("Removing points on a straight line");
    for file in &mut parsed_files {
        // let mut removed_points = 0;
        let coords = &mut file.coords;
        // let old_coords = coords.len();
        for i in 0..=coords.len() - 3 {

            // This can happen because we already removed items
            if i + 2 >= coords.len() {
                break;
            }

            if in_line(&coords[i], &coords[i + 2], &coords[i + 1]) {
                // println!(
                //     "Removing coord: {} {}, which is between {} {} and {} {}",
                //     coords[i + 1].lat,
                //     coords[i + 1].lng,
                //     coords[i].lat,
                //     coords[i].lng,
                //     coords[i + 2].lat,
                //     coords[i + 2].lng
                // );
                coords.remove(i + 1);
                // removed_points += 1;
            }
        }

        //println!("Removed points: {} out of {}, from {}", removed_points, old_coords, file.name);
    }

    println!("Final files: {}", parsed_files.len());
    let mut sum = 0;
    for file in &parsed_files {
        sum += file.coords.len();
    }
    println!("Final points: {}", sum);

    // Final step: write new files
    fs::create_dir_all(output_path_str).unwrap();

    for file in &parsed_files {
        let base_path = Path::new(&file.name);
        let filename = base_path.file_name().unwrap();
        let filename_str = filename.to_str().unwrap().replace(".gpx", ".js");

        let out_path = Path::new(output_path_str);
        let file_out_path = out_path.join(filename_str);

        println!("Creating new file: {}", file_out_path.to_str().unwrap());

        let mut out_file = File::create(file_out_path).unwrap();
        let var_name = filename.to_str().unwrap().replace(".gpx", "");
        out_file.write(b"var ").unwrap();
        out_file.write(var_name.as_bytes()).unwrap();
        out_file.write(b" = [").unwrap();
        for coord in &file.coords {
            let mut coord_str = String::from("[") + &coord.lat.to_string() + "," + &coord.lng.to_string() + "]";
            if coord != file.coords.last().unwrap() {
                coord_str += ",";
            }
            out_file.write(coord_str.as_bytes()).unwrap();
        }
        out_file.write(b"];").unwrap();
    }


}
