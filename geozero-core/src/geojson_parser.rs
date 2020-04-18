use geozero_api::FeatureProcessor;
use serde::Deserialize;
use std::collections::BTreeMap as Map;
use std::io::{BufRead, BufReader, Read, Seek, SeekFrom};

#[derive(Deserialize)]
struct FeatureCollection {
    #[serde(rename = "type")]
    obj_type: FeatureCollectionType,
    features: Vec<Feature>,
}

#[derive(Deserialize)]
struct Feature {
    #[serde(rename = "type")]
    obj_type: FeatureType,
    properties: Map<String, serde_json::Value>,
    geometry: Geometry,
}

#[derive(Deserialize)]
enum FeatureCollectionType {
    FeatureCollection,
}

#[derive(Deserialize)]
enum FeatureType {
    Feature,
}

type Latitude = f32;
type Longitude = f32;
type Coordinate = (Latitude, Longitude);
type Coordinates = Vec<Coordinate>;

#[derive(Deserialize)]
#[serde(tag = "type")]
enum Geometry {
    Point { coordinates: Coordinate },
    MultiPoint { coordinates: Coordinates },
    LineString { coordinates: Coordinates },
    MultiLineString { coordinates: Vec<Coordinates> },
    Polygon { coordinates: Vec<Coordinates> },
    MultiPolygon { coordinates: Vec<Vec<Coordinates>> },
    GeometryCollection,
}

#[derive(Deserialize)]
struct GeojsonPrelude {
    #[serde(rename = "type")]
    obj_type: GeojsonType,
}

#[derive(Deserialize, PartialEq, Debug)]
enum GeojsonType {
    Unknwown,
    FeatureCollection,
    Feature,
    Geometry,
}

fn read_geojson_prelude<R: Read>(reader: R) -> (GeojsonType, usize) {
    let mut geojsontype: GeojsonType = GeojsonType::Unknwown;
    let mut read_ofs = 0;
    let bufreader = BufReader::new(reader);
    let mut buf = Vec::with_capacity(1024);
    if bufreader
        .take(buf.capacity() as u64)
        .read_until(b'[', &mut buf)
        .is_ok()
    {
        if let Ok(prelude) = String::from_utf8(buf) {
            let preludeobj = prelude.replace("[", "0}");
            if let Ok(gj) = serde_json::from_str::<GeojsonPrelude>(&preludeobj) {
                geojsontype = gj.obj_type;
                if geojsontype == GeojsonType::FeatureCollection && prelude.ends_with("[") {
                    read_ofs = prelude.len();
                }
            }
        }
    }
    (geojsontype, read_ofs)
}

pub fn process_geojson<R: Read + Seek + Clone, P: FeatureProcessor + Sized>(
    reader: R,
    mut processor: P,
) -> serde_json::Result<()> {
    let (geojsontype, read_ofs) = read_geojson_prelude(reader.clone());
    dbg!(&geojsontype, read_ofs);
    match geojsontype {
        GeojsonType::FeatureCollection if read_ofs > 0 => {
            // iteratate features
            read_features(reader, read_ofs, &mut processor)?;
        }
        GeojsonType::Feature => {
            // parse feature
            let f: Feature = serde_json::from_reader(reader)?;
            process_feature(&f, &mut processor);
        }
        GeojsonType::Geometry => {
            // parse feature
            let g: Geometry = serde_json::from_reader(reader)?;
            process_geometry(&g, &mut processor);
        }
        _ => {
            dbg!("read_geojson_prelude failed");
        }
    }
    Ok(())
}

fn read_features<R: Read + Seek, P: FeatureProcessor + Sized>(
    mut reader: R,
    read_ofs: usize,
    processor: &mut P,
) -> serde_json::Result<()> {
    reader.seek(SeekFrom::Start(read_ofs as u64)).unwrap();
    dbg!();
    let seq = serde_json::from_reader(reader).into_iter();
    for f in seq {
        process_feature(&f, processor);
    }
    Ok(())
}

fn process_feature<P: FeatureProcessor + Sized>(feat: &Feature, processor: &mut P) {
    dbg!();
    process_geometry(&feat.geometry, processor);
}

fn process_geometry<P: FeatureProcessor + Sized>(geom: &Geometry, processor: &mut P) {
    match geom {
        Geometry::Point { coordinates } => {
            processor.point_begin(0);
            // if multi_dim(processor) {
            // } else {
            processor.pointxy(coordinates.0 as f64, coordinates.1 as f64, 0);
            // }
            processor.point_end();
        }
        Geometry::MultiPoint { coordinates } => {
            processor.multipoint_begin(coordinates.len(), 0);
            // read_points(processor, geometry, 0, coordinates.len());
            processor.multipoint_end();
        }
        Geometry::LineString { coordinates } => {
            processor.line_begin(coordinates.len(), 0);
            // read_points(processor, geometry, 0, coordinates.len());
            processor.line_end(0);
        }
        Geometry::MultiLineString { coordinates } => {
            processor.multiline_begin(coordinates.len(), 0);
            // read_multi_line(processor, geometry, 0);
            processor.multiline_end();
        }
        Geometry::Polygon { coordinates } => {
            processor.poly_begin(coordinates.len(), 0);
            // read_polygon(processor, geometry, false, 0);
            processor.poly_end(0);
        }
        _ => todo!(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use geozero_api::DebugReader;

    const POLYGON: &str = r#"{
    "type": "FeatureCollection",
    "features": [{
        "type": "Feature",
        "geometry": {
            "type": "Polygon",
            "coordinates": [[[30, 10], [40, 40], [20, 40], [10, 20], [30, 10]]]
        },
        "properties": {
            "test1": 1,
            "test2": 1.1,
            "test3": "test3"
        }
    }]
}"#;

    const POLYGON_FEATURE: &str = r#"{
        "type": "Feature",
        "geometry": {
            "type": "Polygon",
            "coordinates": [[[30, 10], [40, 40], [20, 40], [10, 20], [30, 10]]]
        },
        "properties": {
            "test1": 1,
            "test2": 1.1,
            "test3": "test3"
        }
        }"#;

    const POLYGON_GEOMETRY: &str = r#"{
            "type": "Polygon",
            "coordinates": [[[30, 10], [40, 40], [20, 40], [10, 20], [30, 10]]]
        }"#;

    const POINT: &str = r#"{
    "type": "FeatureCollection",
    "features": [{
        "type": "Feature",
        "geometry": {
            "type": "Point",
            "coordinates": [1,1]
        },
        "properties": {
            "test1": 1,
            "test2": 1.1,
            "test3": "test3"
        }
    }]
}"#;

    #[test]
    fn deserialize_fc() -> serde_json::Result<()> {
        let fc: FeatureCollection = serde_json::from_str(POLYGON)?;
        assert_eq!(fc.features.len(), 1);
        if let Geometry::Polygon { coordinates } = &fc.features[0].geometry {
            assert_eq!(coordinates[0].len(), 5);
        } else {
            assert!(false, "Geometry::Polygon expected");
        }

        let fc: FeatureCollection = serde_json::from_str(POINT)?;
        assert_eq!(fc.features.len(), 1);
        if let Geometry::Point { coordinates } = &fc.features[0].geometry {
            assert_eq!(coordinates, &(1.0, 1.0));
        } else {
            assert!(false, "Geometry::Point expected");
        }
        Ok(())
    }

    #[test]
    fn missing_type() -> serde_json::Result<()> {
        let json = r#"{
    "type": "WrongType",
    "features": [{
        "type": "Feature",
        "geometry": {
            "type": "Polygon",
            "coordinates": [[[30, 10], [40, 40], [20, 40], [10, 20], [30, 10]]]
        }
    }]
}"#;
        let fc: Result<FeatureCollection, serde_json::Error> = serde_json::from_str(json);
        assert_eq!(
            fc.err().map(|e| e.to_string()),
            Some(
                "unknown variant `WrongType`, expected `FeatureCollection` at line 2 column 23"
                    .to_string()
            )
        );

        let reader = std::io::Cursor::new(&json);
        let res = process_geojson(reader, DebugReader {});
        assert_eq!(res.err().map(|e| e.to_string()), None);

        let json = r#"{
    "features": [{
        "type": "Feature",
        "geometry": {
            "type": "Polygon",
            "coordinates": [[[30, 10], [40, 40], [20, 40], [10, 20], [30, 10]]]
        },
        "properties": {
        }
    }]
}"#;
        let fc: Result<FeatureCollection, serde_json::Error> = serde_json::from_str(json);
        assert_eq!(
            fc.err().map(|e| e.to_string()),
            Some("missing field `type` at line 11 column 1".to_string())
        );

        let reader = std::io::Cursor::new(&json);
        let res = process_geojson(reader, DebugReader {});
        assert_eq!(res.err().map(|e| e.to_string()), None);

        Ok(())
    }

    #[test]
    fn prelude_reader() -> serde_json::Result<()> {
        let json = r#"{
    "name": "poly_landmarks",
    "type": "FeatureCollection",
    "crs": { "type": "name", "properties": { "name": "urn:ogc:def:crs:OGC:1.3:CRS84" } },
    "features": [
        { "type": "Feature", "properties": { }, "geometry": { "type": "Polygon", "coordinates": [ [ [ -74.020683, 40.691059 ], [ -74.02092, 40.691208 ], [ -74.020466, 40.69124 ], [ -74.020683, 40.691059 ] ] ] } },
        { "type": "Feature", "properties": { }, "geometry": { "type": "Polygon", "coordinates": [ [ [ -73.939885, 40.846674 ], [ -73.940083, 40.846213 ], [ -73.940434, 40.845506 ], [ -73.942533, 40.846403 ], [ -73.94209, 40.84785 ], [ -73.940574, 40.847582 ], [ -73.939686, 40.847381 ], [ -73.939885, 40.846674 ] ] ] } }
    ]
}"#;
        let reader = std::io::Cursor::new(&json);
        let (geojsontype, read_ofs) = read_geojson_prelude(reader);
        assert_eq!(geojsontype, GeojsonType::FeatureCollection);
        assert_eq!(read_ofs, 172);
        Ok(())
    }

    #[test]
    fn feature_collection() -> serde_json::Result<()> {
        // let reader = std::io::Cursor::new(&POLYGON);
        let json = r#"{
    "type": "FeatureCollection",
    "features": [{
        "type": "Feature",
        "geometry": {
            "type": "Polygon",
            "coordinates": [[[30, 10], [40, 40], [20, 40], [10, 20], [30, 10]]]
        },
        "properties": {
            "test1": 1,
            "test2": 1.1,
            "test3": "test3"
        }
    }"#;
        let reader = std::io::Cursor::new(&json);
        let res = process_geojson(reader, DebugReader {});
        assert!(res.is_ok());

        let json = r#"{
    "type": "FeatureCollection",
    "features": [{
        "type": "Feature",
        "geometry": {
            "type": "Polygon",
            "coordinates": [[[30, 10], [40, 40], [20, 40], [10, 20], [30, 10]]]
        },
        "properties": {
            "test1": 1,
            "test2": 1.1,
            "test3": "test3"
        }
    }{
        "type": "Feature",
        "geometry": {
            "type": "Polygon",
            "coordinates": [[[30, 10], [40, 40], [20, 40], [10, 20], [30, 10]]]
        },
        "properties": {
            "test1": 1,
            "test2": 1.1,
            "test3": "test3"
        }
    }"#;
        dbg!("read multi features");
        let reader = std::io::Cursor::new(&json);
        let res = process_geojson(reader, DebugReader {});
        assert!(res.is_ok());

        let json = r#"{
    "type": "FeatureCollection",
    "features": [{
        "type": "Feature",
        "geometry": {
            "type": "Polygon",
            "coordinates": [[[30, 10], [40, 40], [20, 40], [10, 20], [30, 10]]]
        },
        "properties": {
            "test1": 1,
            "test2": 1.1,
            "test3": "test3"
        }
    },{
        "type": "Feature",
        "geometry": {
            "type": "Polygon",
            "coordinates": [[[30, 10], [40, 40], [20, 40], [10, 20], [30, 10]]]
        },
        "properties": {
            "test1": 1,
            "test2": 1.1,
            "test3": "test3"
        }
    }]
}"#;
        dbg!("read multi features");
        let reader = std::io::Cursor::new(&json);
        let res = process_geojson(reader, DebugReader {});
        assert!(res.is_ok());
        assert!(false);
        Ok(())
    }

    #[test]
    fn feature() -> serde_json::Result<()> {
        let reader = std::io::Cursor::new(&POLYGON_FEATURE);
        let res = process_geojson(reader, DebugReader {});
        assert!(res.is_ok());
        Ok(())
    }

    #[test]
    fn geometry() -> serde_json::Result<()> {
        let reader = std::io::Cursor::new(&POLYGON_GEOMETRY);
        let res = process_geojson(reader, DebugReader {});
        assert!(res.is_ok());
        Ok(())
    }
}
