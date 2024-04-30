# powerlog

Rust based tracker of APsystems EZ1 microinverter via local API.
See also: https://github.com/SonnenladenGmbH/APsystems-EZ1-API/blob/main/assets/apsystems-documentation/APsystems%20EZ1%20Local%20API%20-%20Documentation.pdf

I run the tracker on a rpi4 that then samples the microinverter API
every five minutes to track the current output as well as produced daily
and total energy. For the fun of it, I also query some solar irradiation
values as well as the cloud cover near me via open meteo / DWD, see:
https://open-meteo.com/en/docs/dwd-api

The result is written into a sqlite database which I then use to
build a dashboard using obversablehq.
