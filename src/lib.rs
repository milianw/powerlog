// SPDX-License-Identifier: GPL-3.0-or-later

pub mod config {
    pub const LATITUDE: f64 = 52.500;
    pub const LONGITUDE: f64 = 13.493;
    pub const INVERTER_IP: &str = "192.168.178.150";
}

pub mod weather {
    use anyhow::Result;
    use serde::Deserialize;

    #[derive(Deserialize, Debug)]
    struct CurrentWeatherResponse {
        current: CurrentWeather,
    }

    #[derive(Deserialize, Debug)]
    pub struct CurrentWeather {
        pub cloud_cover: f32,

        pub terrestrial_radiation_instant: f32,
        pub direct_radiation_instant: f32,
        pub diffuse_radiation_instant: f32,
        pub shortwave_radiation_instant: f32,
        pub direct_normal_irradiance_instant: f32,
        pub global_tilted_irradiance_instant: f32,
    }

    pub async fn query(client: &reqwest::Client) -> Result<CurrentWeather> {
        let weather_api_url = format!(
            "https://api.open-meteo.com/v1/dwd-icon?latitude={}&longitude={}&current=cloud_cover,shortwave_radiation_instant,direct_radiation_instant,diffuse_radiation_instant,direct_normal_irradiance_instant,global_tilted_irradiance_instant,terrestrial_radiation_instant&tilt=90",
            crate::config::LATITUDE,
            crate::config::LONGITUDE
        );
        let response = client
            .get(weather_api_url)
            .send()
            .await?
            .json::<CurrentWeatherResponse>()
            .await?;
        Ok(response.current)
    }

    #[cfg(test)]
    mod tests {
        #[test]
        fn parse_cloud_cover() {
            let response = r#"
{"latitude":52.52,"longitude":13.419998,"generationtime_ms":0.05900859832763672,"utc_offset_seconds":0,"timezone":"GMT","timezone_abbreviation":"GMT","elevation":38.0,"current_units":{"time":"iso8601","interval":"seconds","cloud_cover":"%","shortwave_radiation_instant":"W/m²","direct_radiation_instant":"W/m²","diffuse_radiation_instant":"W/m²","direct_normal_irradiance_instant":"W/m²","global_tilted_irradiance_instant":"W/m²","terrestrial_radiation_instant":"W/m²"},"current":{"time":"2024-04-16T09:30","interval":900,"cloud_cover":100,"shortwave_radiation_instant":303.7,"direct_radiation_instant":123.5,"diffuse_radiation_instant":180.2,"direct_normal_irradiance_instant":179.1,"global_tilted_irradiance_instant":227.9,"terrestrial_radiation_instant":937.0}}
            "#;

            let weather: crate::weather::CurrentWeatherResponse =
                serde_json::from_str(response).unwrap();
            assert_eq!(weather.current.cloud_cover, 100.0);
            assert_eq!(weather.current.shortwave_radiation_instant, 303.7);
            assert_eq!(weather.current.direct_radiation_instant, 123.5);
            assert_eq!(weather.current.diffuse_radiation_instant, 180.2);
            assert_eq!(weather.current.direct_normal_irradiance_instant, 179.1);
            assert_eq!(weather.current.global_tilted_irradiance_instant, 227.9);
            assert_eq!(weather.current.terrestrial_radiation_instant, 937.0);
        }
    }
}

pub mod inverter {
    use anyhow::Result;
    use const_format::formatcp;
    use serde::Deserialize;

    const INVERTER_URL: &str = formatcp!("http://{}:8050", crate::config::INVERTER_IP);
    const INVERTER_URL_GET_OUTPUTDATA: &str = formatcp!("{INVERTER_URL}/getOutputData");
    const INVERTER_URL_GET_MAXPOWER: &str = formatcp!("{INVERTER_URL}/getMaxPower");
    const INVERTER_URL_GET_ONOFF: &str = formatcp!("{INVERTER_URL}/getOnOff");

    #[derive(Debug)]
    pub struct OutputChannel {
        pub power: f64,
        pub energy_generation_startup: f64,
        pub energy_generation_lifetime: f64,
    }

    #[derive(Debug)]
    pub struct OutputData {
        pub channel1: OutputChannel,
        pub channel2: OutputChannel,
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
            .get(INVERTER_URL_GET_OUTPUTDATA)
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
            .get(INVERTER_URL_GET_MAXPOWER)
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
            .get(INVERTER_URL_GET_ONOFF)
            .send()
            .await?
            .json::<OnOffResponse>()
            .await?
            .data;

        Ok(data.status)
    }

    #[cfg(test)]
    mod tests {
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

pub mod sun {
    pub fn position(time: time::OffsetDateTime) -> sun::Position {
        sun::pos(
            time.unix_timestamp() * 1000,
            crate::config::LATITUDE,
            crate::config::LONGITUDE,
        )
    }
}

pub mod db {
    use anyhow::Result;
    use futures::StreamExt;
    use sea_orm::{ConnectionTrait, FromQueryResult, DbBackend, EntityTrait, Statement};
    use serde::Serialize;

    mod powerlog {
        use sea_orm::entity::prelude::*;
        #[derive(Clone, Debug, PartialEq, DeriveEntityModel)]
        #[sea_orm(table_name = "powerlog")]
        pub struct Model {
            #[sea_orm(primary_key)]
            pub id: i32,

            pub time: time::OffsetDateTime,

            pub power_ch1: f32,
            pub power_ch2: f32,

            pub energy_today_ch1: f32,
            pub energy_today_ch2: f32,

            pub energy_total_ch1: f32,
            pub energy_total_ch2: f32,

            pub max_power: f32,

            #[sea_orm(nullable)]
            pub cloud_cover: f32,

            #[sea_orm(nullable)]
            pub terrestrial_radiation: f32,
            #[sea_orm(nullable)]
            pub direct_radiation: f32,
            #[sea_orm(nullable)]
            pub diffuse_radiation: f32,
            #[sea_orm(nullable)]
            pub shortwave_radiation: f32,
            #[sea_orm(nullable)]
            pub direct_normal_irradiance: f32,
            #[sea_orm(nullable)]
            pub global_tilted_irradiance: f32,

            pub sun_azimuth: f32,
            pub sun_altitude: f32,
        }

        #[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
        pub enum Relation {}

        impl ActiveModelBehavior for ActiveModel {}
    }

    pub async fn setup() -> Result<sea_orm::DatabaseConnection> {
        let db = sea_orm::Database::connect("sqlite://powerlog.sqlite3?mode=rwc").await?;

        let builder = db.get_database_backend();
        let schema = sea_orm::Schema::new(builder);
        let create_table = builder.build(
            schema
                .create_table_from_entity(powerlog::Entity)
                .if_not_exists(),
        );
        db.execute(create_table).await?;

        Ok(db)
    }

    pub async fn insert(
        db: &sea_orm::DatabaseConnection,
        weather: Option<crate::weather::CurrentWeather>,
        sunpos: sun::Position,
        output_data: crate::inverter::OutputData,
        max_power: f64,
        time: time::OffsetDateTime,
    ) -> Result<()> {
        use sea_orm::ActiveValue::{NotSet, Set};

        let mut row = powerlog::ActiveModel {
            // primary key, will be auto generated
            id: NotSet,

            time: Set(time),

            power_ch1: Set(output_data.channel1.power as f32),
            power_ch2: Set(output_data.channel2.power as f32),
            energy_today_ch1: Set(output_data.channel1.energy_generation_startup as f32),
            energy_today_ch2: Set(output_data.channel2.energy_generation_startup as f32),
            energy_total_ch1: Set(output_data.channel1.energy_generation_lifetime as f32),
            energy_total_ch2: Set(output_data.channel2.energy_generation_lifetime as f32),
            max_power: Set(max_power as f32),

            // optional values, see below
            cloud_cover: NotSet,
            terrestrial_radiation: NotSet,
            direct_radiation: NotSet,
            diffuse_radiation: NotSet,
            shortwave_radiation: NotSet,
            direct_normal_irradiance: NotSet,
            global_tilted_irradiance: NotSet,

            sun_azimuth: Set(sunpos.azimuth as f32),
            sun_altitude: Set(sunpos.altitude as f32),
        };

        if let Some(weather) = weather {
            row.cloud_cover = Set(weather.cloud_cover / 100.0);
            row.terrestrial_radiation = Set(weather.terrestrial_radiation_instant);
            row.direct_radiation = Set(weather.direct_radiation_instant);
            row.diffuse_radiation = Set(weather.diffuse_radiation_instant);
            row.shortwave_radiation = Set(weather.shortwave_radiation_instant);
            row.direct_normal_irradiance = Set(weather.direct_normal_irradiance_instant);
            row.global_tilted_irradiance = Set(weather.global_tilted_irradiance_instant);
        }

        use sea_orm::ActiveModelTrait;
        row.insert(db).await?;

        Ok(())
    }

    async fn stream_select<'a, T>(db: &'a sea_orm::DatabaseConnection, query: &str)
     -> Result<impl futures::stream::Stream<Item = T> + 'a>
     where T: FromQueryResult + Send + 'a
    {
        let stream = powerlog::Entity::find()
            .from_raw_sql(Statement::from_string(DbBackend::Sqlite, query))
            .into_model::<T>()
            .stream(db)
            .await?
            .map(|row| row.unwrap());

        Ok(stream)
    }

    #[derive(FromQueryResult, Serialize)]
    pub struct PowerToday {
        #[serde(with = "time::serde::iso8601")]
        time: time::OffsetDateTime,
        power_ch1: f32,
        power_ch2: f32,
        energy_today_ch1: f32,
        energy_today_ch2: f32,

        energy_total_ch1: f32,
        energy_total_ch2: f32,

        max_power: f32,

        cloud_cover: Option<f32>,

        pub terrestrial_radiation: Option<f32>,
        pub direct_radiation: Option<f32>,
        pub diffuse_radiation: Option<f32>,
        pub shortwave_radiation: Option<f32>,
        pub direct_normal_irradiance: Option<f32>,
        pub global_tilted_irradiance: Option<f32>,

        pub sun_azimuth: f32,
        pub sun_altitude: f32,
    }

    pub async fn select_power_today(
        db: &sea_orm::DatabaseConnection,
    ) -> Result<impl futures::stream::Stream<Item = PowerToday> + '_> {
        stream_select::<PowerToday>(db, r#"SELECT * FROM powerlog WHERE time > date('now')"#).await
    }

    #[derive(FromQueryResult, Serialize)]
    pub struct GeneratedByHour {
        hour: String,
        ch1: Option<f32>,
        ch2: Option<f32>,
    }

    pub async fn select_generated_by_hour_today(
        db: &sea_orm::DatabaseConnection,
    ) -> Result<impl futures::stream::Stream<Item = GeneratedByHour> + '_> {
        stream_select::<GeneratedByHour>(db, r#"SELECT
                strftime('%H', time) AS hour,
                (MAX(energy_total_ch1) - (lag(energy_total_ch1) OVER win)) as ch1,
                (MAX(energy_total_ch2) - (lag(energy_total_ch2) OVER win)) as ch2
            FROM powerlog
            WHERE time > date('now')
            GROUP BY hour
            WINDOW win AS (ROWS 1 PRECEDING)"#).await
    }

    #[derive(FromQueryResult, Serialize)]
    pub struct GeneratedByDay {
        date: String,
        ch1: Option<f32>,
        ch2: Option<f32>,
    }

    pub async fn select_generated_by_day(
        db: &sea_orm::DatabaseConnection,
    ) -> Result<impl futures::stream::Stream<Item = GeneratedByDay> + '_> {
        stream_select::<GeneratedByDay>(db, r#"SELECT
                date(time) AS date,
                (MAX(energy_total_ch1) - (lag(energy_total_ch1, 1, 0) OVER win)) as ch1,
                (MAX(energy_total_ch2) - (lag(energy_total_ch2, 1, 0) OVER win)) as ch2
            FROM powerlog
            GROUP BY date
            WINDOW win AS (ROWS 1 PRECEDING)
            ORDER BY date ASC"#).await
    }

}
