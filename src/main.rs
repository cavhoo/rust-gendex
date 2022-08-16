use std::fs;
use std::io::Write;
use std::path::PathBuf;
use std::str::FromStr;
use clap::Parser;
use fancy_regex::Regex;
use wax::Glob;
use wax::LinkBehavior;

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

    // Store pattern in vec for matchers and negations.
    let mut matching_patterns:Vec<String> = vec![];
    let mut negating_patterns:Vec<String> = vec![];

    re_file_patterns.captures_iter(index_group.as_str()).for_each(|file_pattern| {
        let pattern = file_pattern.expect("No pattern found").get(1).expect("No match").as_str().replace("\"", "");

        if pattern.starts_with("!") {
            negating_patterns.push(pattern);
        } else {
            matching_patterns.push(pattern);
        }
    });

    assert!(negating_patterns.len() < 10, "Cannot have more than 10 exlusive patterns.");

    // Loop over match patterns and filter files inside directory.
    for pattern in matching_patterns.iter() {
        let file_matches = find_files_matching(&pattern, directory_path.to_str().unwrap(), negating_patterns.as_slice());

        let mut index_file = fs::OpenOptions::new()
            .write(true)
            .append(true)
            .open(args.file.clone())
            .unwrap();

        for file in file_matches.iter() {
            println!("File: {  }", file);
            write!(index_file, "export * from \"{  }\"\n", file.replace(".ts", "")).expect("Could not write to file.");
        }
    }



    // Extracts the formatter function from the index declaration.
}

fn find_files_matching(pattern: &str, source_folder: &str, excluded_patterns: &[String]) -> Vec<String> {
    println!("Using Pattern: {  }", trim_first_character(pattern));
    println!("Using Path: {  }", source_folder);
    let file_glob = Glob::new(trim_first_character(pattern)).unwrap();


    let mut files = vec![];
    for file in file_glob.walk_with_behavior(source_folder, LinkBehavior::ReadFile).not(["**/testUtils", "**/__tests__", "types.d.ts", "index.ts"]).unwrap() {
        let file_path = file.unwrap().path().to_str().unwrap().replace(source_folder, ".");
        files.push(file_path)
    }

    files
}

fn trim_first_character(file_pattern: &str) -> &str {
    if file_pattern.starts_with("./") {
        let mut chars = file_pattern.chars();
        chars.next();
        chars.next();
        return chars.as_str()
    } else {
        return file_pattern
    }
}
