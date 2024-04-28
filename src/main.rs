// SPDX-License-Identifier: GPL-3.0-or-later

use anyhow::Result;
use std::time::Duration;

use powerlog::db;
use powerlog::inverter;
use powerlog::sun;
use powerlog::weather;

#[tokio::main]
async fn main() -> Result<()> {
    env_logger::init();

    let time = time::OffsetDateTime::now_utc();

    let db = db::setup();

    // setup http clients
    let client = reqwest::Client::builder()
        .timeout(Duration::new(10, 0))
        .gzip(true)
        .brotli(true)
        .deflate(true)
        .build()?;

    // fail early when the inverter is offline
    let on_off = match inverter::on_off(&client).await {
        Err(e) => {
            if let Some(e) = e.downcast_ref::<reqwest::Error>() {
                if e.is_connect() || e.is_timeout() {
                    println!("inverter is offline: {:?}", e);
                    return Ok(());
                }
            }
            panic!("inverter request failure: {:?}", e);
        }
        Ok(on_off) => on_off,
    };

    // access inverter API
    let client_copy = client.clone();
    let inverter_requests = tokio::spawn(async move {
        (
            inverter::output_data(&client_copy).await,
            inverter::max_power(&client_copy).await,
        )
    });

    // access weather API
    let client_copy = client.clone();
    let weather_request = tokio::spawn(async move { weather::query(&client_copy).await });

    // await all requests

    // gracefully handle failures of weather api access
    let weather = match weather_request.await? {
        Ok(weather) => Some(weather),
        Err(err) => {
            eprintln!("{:?}", err);
            None
        }
    };

    // don't do anything if inverter read outs fail which happens at night time when the device is off
    let (output_data, max_power) = inverter_requests.await?;

    // handle accumulated data
    let output_data = output_data?;
    let max_power = max_power?;
    let sunpos = sun::position(time);
    println!("weather: {weather:?}, output data: {output_data:?}, max power: {max_power} on/off: {on_off:?}, sun: {sunpos:?}");

    // insert data
    let db = db.await?;
    db::insert(&db, weather, sunpos, output_data, max_power, time).await?;

    Ok(())
}
