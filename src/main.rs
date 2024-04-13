mod config {
    use const_format::formatcp;

    const LATITUDE: &str = "52.500";
    const LONGITUDE: &str = "13.493";
    const PIRATEWEATHER_API_KEY: &str = "...";
    const INVERTER_IP: &str = "192.168.178.150";
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

    fn to_output_data(data: RawOutputData) -> OutputData {
        OutputData {
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
        }
    }

    pub async fn output_data(client: &reqwest::Client) -> Result<OutputData> {
        let data = client
            .get(config::INVERTER_URL_GET_OUTPUTDATA)
            .send()
            .await?
            .json::<OutputDataResponse>()
            .await?
            .data;

        Ok(to_output_data(data))
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

    #[derive(Deserialize, Debug, Eq, PartialEq)]
    pub enum Status {
        #[serde(rename = "0")]
        On,
        #[serde(rename = "1")]
        Off,
    }

    #[derive(Deserialize, Debug)]
    struct OnOff {
        status: Status,
    }

    #[derive(Deserialize, Debug)]
    struct OnOffResponse {
        data: OnOff,
    }
    pub async fn on_off(client: &reqwest::Client) -> Result<Status> {
        let data = client
            .get(config::INVERTER_URL_GET_ONOFF)
            .send()
            .await?
            .json::<OnOffResponse>()
            .await?
            .data;

        Ok(data.status)
    }

    #[cfg(test)]
    mod tests {
        use serde_json;

        #[test]
        fn parse_output_data() {
            let response = r#"
{
    "data": {
        "p1": 1,
        "e1": 2,
        "te1": 3,
        "p2": 4,
        "e2": 5,
        "te2": 6
    },
    "message": "SUCCESS",
    "deviceId":"E07000000001"
}
            "#;

            let response: crate::inverter::OutputDataResponse =
                serde_json::from_str(response).unwrap();
            let data = crate::inverter::to_output_data(response.data);
            assert_eq!(data.channel1.power, 1_f64);
            assert_eq!(data.channel1.energy_generation_startup, 2_f64);
            assert_eq!(data.channel1.energy_generation_lifetime, 3_f64);
            assert_eq!(data.channel2.power, 4_f64);
            assert_eq!(data.channel2.energy_generation_startup, 5_f64);
            assert_eq!(data.channel2.energy_generation_lifetime, 6_f64);
        }

        #[test]
        fn parse_max_power() {
            let response = r#"
{
    "data": {
        "maxPower": "600"
    },
    "message": "SUCCESS",
    "deviceId":"E07000000001"
}
            "#;

            let response: crate::inverter::MaxPowerResponse =
                serde_json::from_str(response).unwrap();
            assert_eq!(response.data.maxPower.parse::<f64>().unwrap(), 600_f64);
        }

        #[test]
        fn parse_on_off() {
            let response = r#"
{
    "data": {
        "status": "0"
    },
    "message": "SUCCESS",
    "deviceId":"E07000000001"
}
            "#;

            let response: crate::inverter::OnOffResponse = serde_json::from_str(response).unwrap();
            assert_eq!(response.data.status, crate::inverter::Status::On);
        }
    }
}

use anyhow::Result;
use std::time::Duration;

#[tokio::main]
async fn main() -> Result<()> {
    env_logger::init();

    // setup http clients
    let client = reqwest::Client::builder()
        .timeout(Duration::new(5, 0))
        .gzip(true)
        .brotli(true)
        .deflate(true)
        .build()?;

    // access weather API
    let client_copy = client.clone();
    let weather_request = tokio::spawn(async move { weather::cloud_cover(&client_copy).await });

    // access inverter API
    let client_copy = client.clone();
    let inverter_requests = tokio::spawn(async move {
        (
            inverter::output_data(&client_copy).await,
            inverter::max_power(&client_copy).await,
            inverter::on_off(&client_copy).await,
        )
    });

    // await all requests
    // gracefully handle failures of pirateweather access
    let cloud_cover = weather_request.await?.unwrap_or(0_f64);
    // don't do anything if inverter read outs fail which happens at night time when the device is off
    let (output_data, max_power, on_off) = inverter_requests.await?;

    // handle accumulated data
    let output_data = output_data?;
    let max_power = max_power?;
    let on_off = on_off?;
    println!("cover: {cloud_cover}, output data: {output_data:?}, max power: {max_power} on/off: {on_off:?}");

    Ok(())
}
