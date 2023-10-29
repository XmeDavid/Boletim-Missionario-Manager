use std::io::Read;
use std::{error::Error, io::Write};
use std::fs::File;
use std::env;
use std::process;
use scraper::{Html, Selector};
use tokio;
use regex::Regex;
use serde::{Serialize, Deserialize};

#[derive(Debug, Serialize, Deserialize, Clone)]
struct Boletim{
    id: String,
    name: String,
    url: String,
    size: String,
    file_type: String,
}

fn save_to_file(file_name:&str, data:Vec<Boletim>){
    let mut file:File = File::create(file_name).unwrap();
    let json = serde_json::to_string(&data).unwrap();
    let _ = file.write_all(json.as_bytes());
}

async fn fetch_boletins() -> Result<(), Box<dyn Error>> {
    // Create an HTTP client
    let client = reqwest::Client::new();

    let url = "https://recursos.adventistas.org.pt/escolasabatina/videos/boletim-missionario-4-o-trimestre-de-2023";

    // Send an HTTP GET request and get the response
    let response = client.get(url).send().await?;

    // Check if the request was successful
    if !response.status().is_success() {
        return Err("Request failed with a non-success status code".into());
    }

    // Read the response body as text
    let response_text = response.text().await?;

    let document = Html::parse_document(&response_text);

    // Define a CSS selector to select the table rows
    let tr_selector = Selector::parse("table tr").unwrap();

    // Find all the table rows in the HTML
    let tr_elements = document.select(&tr_selector);

    // Create a vector to store the table data as objects
    let mut table_data: Vec<Boletim> = Vec::new();

    let re = Regex::new(r"^[a-zA-Z0-9]+\s*").unwrap();

    // Iterate over the table rows
    for row in tr_elements {
        // Define a CSS selector to select the table cells (td elements) within the row
        let td_selector = Selector::parse("td").unwrap();
        let mut td_elements = row.select(&td_selector);
        
        // Extract the values for column1 and column2
        
        let raw_name:String = td_elements.next().map(|e| e.text().collect()).unwrap_or_default();
        let formatted_name: String = raw_name.replace("\n", "").trim().to_string();
        let captures = re.captures(&formatted_name);
        let title: String;
        let file_type: String;

        match captures {
            Some(captures) => {
                file_type = captures.get(0).unwrap().as_str().trim().to_string();
                title = formatted_name.replace(captures.get(0).unwrap().as_str(), "").trim().to_string();
            }
            None => {
                title = formatted_name;
                file_type = String::new();
            }
        }


        let size:String = td_elements.next().map(|e| e.text().collect()).unwrap_or_default();
        let url_raw:String = td_elements.next().map(|e| e.inner_html()).unwrap_or_default();
        
        let document = Html::parse_document(url_raw.as_str());

        // Define a CSS selector to select the <a> element
        let a_selector = Selector::parse("a").unwrap();

        let mut url = String::new();
        // Find the first <a> element in the HTML
        if let Some(a_element) = document.select(&a_selector).next() {
            // Extract the value of the 'href' attribute
            if let Some(href) = a_element.value().attr("href") {
                url = href.to_string();
            } else {
                println!("No 'href' attribute found in the <a> element.");
            }
        } else {
            continue;
        }

        let parts: Vec<&str> = title.splitn(2, '-').map(|s| s.trim()).collect();
        let name: String;
        let mut id: String;
        
        if parts.len() == 2 {
            id = parts[0].to_string();
            name = parts[1].to_string();
            
            if name.contains("(com legendas)") || name.contains("(Com legendas)") {
                id = format!("1{}", id);
            }


            println!("id: {}", id);
            println!("name: {}", name);
        } else {
            id = String::new();
            name = String::new();
            println!("Invalid input string format. Expected 'id - name'.");
        }

        // Create a new TableData object and add it to the vector
        let data = Boletim {
            id,
            name,
            url,
            size,
            file_type,
        };
        println!("------------------");
        println!("ID: {}",data.id);
        println!("Type: {}",data.file_type);
        println!("Name: {}",data.name);
        println!("Size: {}",data.size);
        println!("URL: {}",data.url);

        table_data.push(data);
    }
    save_to_file("boletins.json",table_data);
    Ok(())
}


async fn download_file(url: &str, output_path: &str) -> Result<(), Box<dyn std::error::Error>> {
    println!("Getting Data...");
    let response = reqwest::get(url)
        .await?
        .error_for_status()?;
    println!("Got Data, Writing to file...");
    let bytes = response.bytes().await?;
    let _ = std::fs::write(output_path, &bytes);
    
    println!("Downloaded file to: {}", output_path);
    Ok(())
}


async fn download(id:String, path:&str) {
    println!("---------------");
    println!("Starting Download of Boletim with id: {}",id);
    let boletins = get_boletins();
    let boletim:Boletim = boletins.iter().find(|boletim| boletim.id == id).unwrap_or_else(|| {
        process::exit(1)
    }).clone();

    let output_path = path.to_owned() + &boletim.name.to_owned() + "." + &boletim.file_type.to_owned();
    let _result = download_file(boletim.url.as_str(), output_path.as_str()).await;


}

async fn download_all(path:&str){
    let boletins = get_boletins();
    println!("Downloading all");
    for boletim in boletins {
        download(boletim.id,path).await;
    }
}

fn get_boletins() -> Vec<Boletim>{
    //Read boletins.json and turn into Vec<Boletim>
    let file_name = "boletins.json";
    let mut file:File = File::open(file_name).unwrap();
    let mut data = String::new();
    file.read_to_string(&mut data).unwrap();


    return serde_json::from_str(&data).unwrap();

}


#[tokio::main]
async fn main(){
    let list_option = "-l".to_string();
    let get_option = "-g".to_string();
    let download_all_option = "-da".to_string();
    let download_option = "-d".to_string();
    let args: Vec<String> = env::args().collect();

    if args.contains(&list_option){
        let boletins = get_boletins();

        for boletin in boletins{
            println!("{} - {} | {}",boletin.id, boletin.name, boletin.size);
        }
        return;
    }
    if args.contains(&get_option){
        println!("Fetching boletins...");
        if let Err(err) = fetch_boletins().await {
            println!("Error: {}", err);
        }
        return;
    }
    if args.contains(&download_all_option){
        download_all("").await;
        return;
    }
    if args.contains(&download_option){
        
        let target = "-d";
        let file_id: &str;
        if let Some(index) = args.iter().position(|s| s == target) {
            if index + 1 < args.len() {
                file_id = &args[index + 1];
                
            } else {
                println!("Please write the boletin id after the '-d' argument, to select which one to download.");
                return;
            }
        }
        else{
            return;
        }
        download(file_id.to_string(), "").await;
        return;

    }
    print!("No Valid arguments found.")
}