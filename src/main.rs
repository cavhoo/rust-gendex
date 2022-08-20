use std::{fs, result};
use std::io::{Write, BufReader, BufRead};
use std::io::Read;
use std::path::PathBuf;
use glob::glob;
use clap::Parser;
use fancy_regex::Regex;

#[derive(Parser, Debug)]
#[clap(author, version, about, long_about = None)]
struct Args {
    /// The path to the index.ts file that serves as the root export.
    #[clap(short, long, value_parser)]
    file: String,

    /// Honor folder index.ts files
    #[clap(long, value_parser)]
    allow_folder_exports: bool,
}

fn main() {
    // Read cli arguments
    let args = Args::parse();

    let indexfile_content = fs::read_to_string(args.file.clone()).expect("Could not read provided index.ts file. Please make sure the file exists.");

    let src_path = PathBuf::from(args.file.clone());
    let directory_path = src_path.parent().unwrap();


    // RegEx for checking if provided file contains an index declaration.
    let re_index_declaration = Regex::new(r"([^\r\n]*)@index\(([^\r\n]+)\)[^\r\n]*(?=[\r\n]|$)").unwrap();

    let index_declaration_result = re_index_declaration.captures(&indexfile_content);
    let captures = index_declaration_result.expect("Error executing regex").expect("No match");

    assert!(captures.len() > 1, "No valid match for index declaration found.");

    let index_group = captures.get(2).expect("no group");

    // Extract all file pattern regexes inside the index declarattion.
    let re_file_patterns = Regex::new(r#"(\"[^\r\n\s]+\"(?=[,\]]))"#).unwrap();
    let re_export_style = Regex::new("`.+`").unwrap();


    // Store pattern in vec for matchers and negations.
    let mut inclusion_patterns:Vec<String> = vec![];
    let mut exclusion_patterns:Vec<String> = vec![];

    re_file_patterns.captures_iter(index_group.as_str()).for_each(|file_pattern| {
        let pattern = file_pattern.expect("No pattern found").get(1).expect("No match").as_str().replace("\"", "");

        if pattern.starts_with("!") {
            exclusion_patterns.push(pattern);
        } else {
            inclusion_patterns.push(pattern);
        }
    });

    let captures_export_template = re_export_style.captures(index_group.as_str()).expect("RegEx Error").expect("No Match");
    let export_template_match = captures_export_template.get(1).expect("No valid export template found.");
    let export_template_captures = Regex::new(r#"\$\{.+\}"#).unwrap().captures(export_template_match.as_str()).expect("RegEx Error").expect("No Match found.");

    let export_template_string = export_template_captures.get(1).expect("No Group");

    let mut index_file = fs::OpenOptions::new()
        .read(true)
        .write(true)
        .truncate(true)
        .open(args.file.clone())
        .unwrap();

    let mut final_content:Vec<String> = vec![];

    let index_lines: Vec<_> = indexfile_content.split("\n").collect();
    final_content.push(index_lines.get(0).unwrap().to_string());

    // Loop over match patterns and filter files inside directory.
    for pattern in inclusion_patterns.iter() {
        let file_matches = find_files_matching(&pattern, directory_path.to_str().unwrap(), exclusion_patterns.as_slice());

        for file in file_matches.iter() {
            final_content.push(format!("export * from \"{  }\"", file.replace(".ts", "")));
            //writeln!(index_file, "export * from \"{  }\"", file.replace(".ts", "")).expect("Could not write to file.");
        }
    }

    for line in final_content.iter() {
        println!("Writing { }", line);
        writeln!(index_file, "{  }", line).expect("Could not write to file.");
    }

}

fn find_files_matching(file_pattern: &str, source_folder: &str, exclusion_patterns: &[String]) -> Vec<String> {
    // Vector of file paths to be added to the index file.
    let mut files = vec![];
    // Regex to remove glob negation to be able to use it in a regex.
    let cleaning_regex = Regex::new("!?\\**").unwrap();
    // Concat the source_folder with the file glob pattern to make sure we are running the right folder.
    let glob_path = format!("{ }/{ }", source_folder, trim_first_character(file_pattern));
    for file in glob(&glob_path).expect("Invalid glob pattern") {
        match file {
            Ok(path) => {
                let mut excluded = false;
                for pattern in exclusion_patterns {
                    let cleaned_excluded = cleaning_regex.replace(&pattern, ".+");
                    let excluded_pattern = Regex::new(&cleaned_excluded.as_ref()).unwrap();
                    let match_result = excluded_pattern.is_match(&path.to_str().unwrap());
                    if match_result.is_ok() {
                        let matches = match_result.unwrap();
                        if matches {
                            excluded = true;
                        }
                    }
                }

                // If file is not exlusion then add it to the list of files.
                if !excluded {
                    files.push(path.display().to_string().replace(source_folder, "."))
                }
            },
            Err(e) => println!("{:?}", e)
        }
    }

    files
}

/// Removes the leading ./ of local file paths
///
///  # Arguments
///  * `file_path` - relative file path to be cleaned from local reference.
fn trim_first_character(file_path: &str) -> &str {
    if file_path.starts_with("./") {
        let mut chars = file_path.chars();
        chars.next();
        chars.next();
        return chars.as_str()
    } else {
        return file_path
    }
}
