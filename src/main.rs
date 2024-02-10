use std::{ffi::CString, fs::File, io::Write, thread, time::Duration};

use config::Config;
use enigo::{Enigo, KeyboardControllable};
use pcsc::*;

type Reader = CString;

mod config;

const APDU_GET_UID: &[u8] = &[0xff, 0xca, 0x00, 0x00, 0x00];
// const APDU_GET_HISTORY: &[u8] = &[0xff, 0xca, 0x00, 0x00, 0x00];
// const APDU_GET_ID: &[u8] = &[0xff, 0xca, 0xf0, 0x00, 0x00];
// const APDU_GET_NAME: &[u8] = &[0xff, 0xca, 0xf1, 0x00, 0x00];
// const APDU_GET_CS: &[u8] = &[0xff, 0xca, 0xf2, 0x00, 0x00];
// const APDU_GET_TYPE: &[u8] = &[0xff, 0xca, 0xf3, 0x00, 0x00];
// const APDU_GET_TYPENAME: &[u8] = &[0xff, 0xca, 0xf4, 0x00, 0x00];

const APDU_START_TRANSPARENT_SESSION: &[u8] = &[0xff, 0xc2, 0x00, 0x00, 0x02, 0x81, 0x00, 0x00];
const APDU_END_TRANSPARENT_SESSION: &[u8] = &[0xff, 0xc2, 0x00, 0x00, 0x02, 0x82, 0x00, 0x00];
const APDU_USE_FELICA: &[u8] = &[0xff, 0xc2, 0x00, 0x02, 0x04, 0x8f, 0x02, 0x03, 0x01, 0x00];

fn main() {
    let config = config::get_config();

    loop {
        match main_loop(&config) {
            Err(e) => eprintln!("{}", e),
            Ok(_) => { /* continue */ }
        }

        thread::sleep(Duration::from_millis(1000));
    }
}

fn trigger_login(config: &Config, uid: &[u8]) -> Result<(), String> {
    {
        let mut file = match File::create(&config.felica_file) {
            Ok(f) => f,
            Err(e) => return Err(format!("Failed to open felica file: {}", e)),
        };

        let uid_string = hex::encode(uid);

        if let Err(e) = writeln!(file, "{:f>16}", uid_string) {
            return Err(format!("Failed to write to felica file: {}", e))
        };

        if let Err(e) = file.flush() {
            return Err(format!("Failed to flush file: {}", e))
        };
    }

    // a little delay to let windows recognize that the file has changed
    thread::sleep(Duration::from_millis(200));
    let mut enigo = Enigo::new();
    enigo.key_down(config.login_key);
    thread::sleep(Duration::from_millis(1000));
    enigo.key_up(config.login_key);

    Ok(())
}

fn main_loop(config: &Config) -> Result<(), String> {
    let ctx = match Context::establish(Scope::User) {
        Ok(ctx) => ctx,
        Err(err) => {
            panic!("Failed to establish context: {}", err);
        }
    };

    let reader = get_reader(&ctx)?;

    println!("Using reader: {:?}", reader);

    // Connect to the card.
    loop {
        let card = connect(&ctx, &reader)?;

        if let Some(card) = card {
            let id = get_card_id(&card)?;
            println!("card connected: {:0>2x?}", id);
            trigger_login(&config, &id)?;
            // wait 2 seconds before scanning again
            thread::sleep(Duration::from_millis(2000));
            hold_card(&card)?;
            println!("card disconnected");
        } else {
            thread::sleep(Duration::from_millis(200))
        }
    }
}

fn transmit(card: &Card, cmd: &[u8]) -> Result<Vec<u8>, String> {
    let mut result_buffer = [0; MAX_BUFFER_SIZE];
    match card.transmit(cmd, &mut result_buffer) {
        Ok(r) => {
            print_error_info(r);
            Ok(r.to_vec())
        },
        Err(err) => return Err(format!("Failed to transmit APDU command to card: {}", err))
    }
}

fn print_error_info(response: &[u8]) {
    let response = hex::encode(response);

    let predefined_error = match &response[..] {
        // not exaustive
        "6401" => "no response",
        "6700" => "invalid length",
        "6a81" => "invalid instruction",
        "6f00" => "unexpected error",
        _ => return
    };

    eprintln!("The last transmission returned an error: {}", predefined_error);
}

fn get_card_id(card: &Card) -> Result<Vec<u8>, String> {
    let mut result = transmit(card, APDU_GET_UID)?;

    if let Some(id) = get_felica_id(card)? {
        result = id;
    }

    let length = result.len() - 2;
    assert!(result[length] == 0x90u8);
    assert!(result[length + 1] == 0u8);

    Ok(result[..length].to_vec())
}

fn get_felica_id(card: &Card) -> Result<Option<Vec<u8>>, String> {
    transmit(card, APDU_START_TRANSPARENT_SESSION)?;
    let use_felica_result = transmit(card, APDU_USE_FELICA)?;
    transmit(card, APDU_END_TRANSPARENT_SESSION)?;
    // debug(card);

    if hex::encode(use_felica_result).starts_with("c003009000") {
        let id = transmit(card, APDU_GET_UID)?;
        return Ok(Some(id))
    } else {
        return Ok(None)
    }
}

fn get_reader(ctx: &Context) -> Result<Reader, String> {
    // List available readers.
    let mut readers_buf = [0; 2048];
    let mut readers = match ctx.list_readers(&mut readers_buf) {
        Ok(readers) => readers,
        Err(err) => {
            return Err(format!("Failed to establish context: {}", err));
        }
    };

    // Use the first reader.
    let reader = match readers.next() {
        Some(reader) => reader,
        None => {
            return Err("No readers are connected.".to_owned());
        }
    };

    Ok(reader.to_owned())
}

fn connect(ctx: &Context, reader: &Reader) -> Result<Option<Card>, String> {
    // Connect to the card.
    let card = match ctx.connect(&reader, ShareMode::Shared, Protocols::ANY) {
        Ok(card) => card,
        Err(Error::NoSmartcard) | Err(Error::RemovedCard) => return Ok(None),
        Err(err) => return Err(format!("Failed to connect to card: {}", err))
    };

    Ok(Some(card))
}

fn hold_card(card: &Card) -> Result<(), String> {
    loop {
        let mut names_buffer = [0u8; MAX_BUFFER_SIZE];
        let mut atr_buffer = [0u8; MAX_BUFFER_SIZE];

        match card.status2(&mut names_buffer, &mut atr_buffer) {
            Ok(_) => { /* do nothing */},
            Err(Error::NoSmartcard) | Err(Error::RemovedCard) => return Ok(()),
            Err(e) => return Err(format!("Failed to get card status: {}", e))
        };

        thread::sleep(Duration::from_millis(200));
    }
}

// fn debug(card: &Card) {
//     debug_transmit(card, APDU_GET_UID, "uid");
//     debug_transmit(card, APDU_GET_HISTORY, "history");
//     debug_transmit(card, APDU_GET_ID, "id");
//     debug_transmit(card, APDU_GET_NAME, "name of card");
//     debug_transmit(card, APDU_GET_TYPE, "type of card");
//     debug_transmit(card, APDU_GET_TYPENAME, "name of card type");
//     debug_transmit(card, APDU_GET_CS, "communication speed");
// }

// fn debug_transmit(card: &Card, cmd: &[u8], name: &str) {
//     match transmit(card, cmd) {
//         Ok(r) => println!("{}: {:0>2x?}", name, r),
//         Err(e) => println!("Err: {}", e),
//     }
// }
