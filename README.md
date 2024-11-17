# gpx2js-rs
Converts a bunch of GPX files into JS files / arrays

# NOTE
This is just a small testing program, nothing serious. But maybe it will help your inspiration :)

I use it in combination with [garmin-connect-export](https://github.com/pe-st/garmin-connect-export)

# Example Usage

```sh
$ mkdir -p gpx_all gpx_js
# Download the last 50 activities
$ python garmin-connect-export/gcexport.py -c 50 -f gpx -d gpx_all --username <garmin_username> --password <garmin_password>
# Convert into JS files
$ gpx2js-rs -i gpx_all -o gpx_js

# Optional: use a skip file, to exclude activities with known broken/weird GPS coordinates
$ echo "activity_1234567.gpx" > skip.txt
$ gpx2js-rs -i gpx_all -o gpx_js -s skip.txt
```

# Screenshots

With some more bash magic and leaflet, you can achieve something like this

![screenshot](http://www.buschinski.de/img-misc/walkmap.png)
