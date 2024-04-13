mod config {
    use const_format::formatcp;

    const LATITUDE: &str = "52.500";
    const LONGITUDE: &str = "13.493";
    const PIRATEWEATHER_API_KEY: &str = "...";
    const INVERTER_IP: &str = "192.168.178.63";
    const INVERTER_URL: &str = formatcp!("http://{INVERTER_IP}:8050");

    pub const PIRATEWEATHER_URL : &str = formatcp!("https://api.pirateweather.net/forecast/{PIRATEWEATHER_API_KEY}/{LATITUDE},{LONGITUDE}?units=si&exclude=minutely,hourly,daily,alerts");
    pub const INVERTER_URL_GET_OUTPUTDATA: &str = formatcp!("{INVERTER_URL}/getOutputData");
    pub const INVERTER_URL_GET_MAXPOWER: &str = formatcp!("{INVERTER_URL}/getMaxPower");
    pub const INVERTER_URL_GET_ONOFF: &str = formatcp!("{INVERTER_URL}/getOnOff");
}

mod weather {
    use crate::config;
    use anyhow::Result;
    use serde::Deserialize;

    #[derive(Deserialize, Debug)]
    struct Weather {
        currently: CurrentWeather,
    }

    #[derive(Deserialize, Debug)]
    #[allow(non_snake_case)]
    struct CurrentWeather {
        cloudCover: f64,
    }

    pub async fn cloud_cover(client: &reqwest::Client) -> Result<f64> {
        let response = client
            .get(config::PIRATEWEATHER_URL)
            .send()
            .await?
            .json::<Weather>()
            .await?;
        Ok(response.currently.cloudCover)
    }

    #[cfg(test)]
    mod tests {
        use serde_json;

        #[test]
        fn parse_cloud_cover() {
            let response = r#"
{
  "latitude": 52.5,
  "longitude": 13.493,
  "timezone": "Europe/Berlin",
  "offset": 2.0,
  "elevation": 37,
  "currently": {
    "time": 1712930580,
    "summary": "Cloudy",
    "icon": "cloudy",
    "nearestStormDistance": 0,
    "nearestStormBearing": 0,
    "precipIntensity": 0.0,
    "precipProbability": 0.0,
    "precipIntensityError": 0.0,
    "precipType": "none",
    "temperature": 18.52,
    "apparentTemperature": 19.63,
    "dewPoint": 9.63,
    "humidity": 0.56,
    "pressure": 1021.98,
    "windSpeed": 5.09,
    "windGust": 7.88,
    "windBearing": 255,
    "cloudCover": 0.12,
    "uvIndex": 2.4,
    "visibility": 16.09,
    "ozone": 328.39
  },
  "flags": {
    "sources": [
      "ETOPO1",
      "gfs",
      "gefs"
    ],
    "sourceTimes": {
      "gfs": "2024-04-12 06:00:00",
      "gefs": "2024-04-12 06:00:00"
    },
    "nearest-station": 0,
    "units": "si",
    "version": "V1.5.6"
  }
}
            "#;

            let weather: crate::weather::Weather = serde_json::from_str(response).unwrap();
            assert_eq!(weather.currently.cloudCover, 0.12);
        }
    }
}

mod inverter {
    use crate::config;
    use anyhow::Result;
    use serde::Deserialize;

    #[derive(Debug)]
    pub struct OutputChannel {
        power: f64,
        energy_generation_startup: f64,
        energy_generation_lifetime: f64,
    }

    #[derive(Debug)]
    pub struct OutputData {
        channel1: OutputChannel,
        channel2: OutputChannel,
    }

    #[derive(Deserialize, Debug)]
    struct RawOutputData {
        p1: f64,
        p2: f64,
        e1: f64,
        e2: f64,
        te1: f64,
        te2: f64,
    }

    #[derive(Deserialize, Debug)]
    struct OutputDataResponse {
        data: RawOutputData,
    }

    pub async fn output_data(client: &reqwest::Client) -> Result<OutputData> {
        let data = client
            .get(config::INVERTER_URL_GET_OUTPUTDATA)
            .send()
            .await?
            .json::<OutputDataResponse>()
            .await?
            .data;

        Ok(OutputData {
            channel1: OutputChannel {
                power: data.p1,
                energy_generation_startup: data.e1,
                energy_generation_lifetime: data.te1,
            },
            channel2: OutputChannel {
                power: data.p2,
                energy_generation_startup: data.e2,
                energy_generation_lifetime: data.te2,
            },
        })
    }

    #[derive(Deserialize, Debug)]
    #[allow(non_snake_case)]
    struct RawMaxPower {
        maxPower: String,
    }

    #[derive(Deserialize, Debug)]
    struct MaxPowerResponse {
        data: RawMaxPower,
    }

    pub async fn max_power(client: &reqwest::Client) -> Result<f64> {
        let data = client
            .get(config::INVERTER_URL_GET_MAXPOWER)
            .send()
            .await?
            .json::<MaxPowerResponse>()
            .await?
            .data;

        Ok(data.maxPower.parse()?)
    }

    #[derive(Deserialize, Debug)]
    struct OnOff {
        status: String,
    }

    #[derive(Deserialize, Debug)]
    struct OnOffResponse {
        data: OnOff,
    }

    pub async fn on_off(client: &reqwest::Client) -> Result<bool> {
        let data = client
            .get(config::INVERTER_URL_GET_ONOFF)
            .send()
            .await?
            .json::<OnOffResponse>()
            .await?
            .data;

        Ok(data.status.parse()?)
    }
}

use anyhow::Result;
use std::time::Duration;

#[tokio::main]
async fn main() -> Result<()> {
    let client = reqwest::Client::builder()
        .connect_timeout(Duration::new(5, 0))
        .build()?;

    let cloud_cover = weather::cloud_cover(&client);
    let output_data = inverter::output_data(&client);
    let max_power = inverter::max_power(&client);
    let on_off = inverter::on_off(&client);

    let cloud_cover = cloud_cover.await?;
    let output_data = output_data.await?;
    let max_power = max_power.await?;
    let on_off = on_off.await?;
    println!("cover: {cloud_cover}, output data: {output_data:?}, max power: {max_power} on/off: {on_off}");

    Ok(())
}
