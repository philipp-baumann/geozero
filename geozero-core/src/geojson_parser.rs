use serde::de::{MapAccess, SeqAccess, Visitor};
use serde::{Deserialize, Deserializer};
use std::collections::BTreeMap as Map;
use std::fmt;
use std::marker::PhantomData;

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub struct FeatureCollection {
    #[serde(rename = "type")]
    pub obj_type: FeatureCollectionType,
    pub features: Vec<Feature>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Feature {
    #[serde(rename = "type")]
    pub obj_type: FeatureType,
    #[serde(deserialize_with = "deserialize_properties")]
    pub properties: Map<String, serde_json::Value>,
    pub geometry: Geometry,
}

#[derive(Deserialize)]
pub enum FeatureCollectionType {
    FeatureCollection,
}

#[derive(Deserialize)]
pub enum FeatureType {
    Feature,
}

pub type Latitude = f32;
pub type Longitude = f32;
pub type Coordinate = (Latitude, Longitude);
pub type Coordinates = Vec<Coordinate>;

#[derive(Deserialize)]
#[serde(tag = "type")]
pub enum Geometry {
    Point {
        coordinates: Coordinate,
    },
    MultiPoint {
        coordinates: Coordinates,
    },
    LineString {
        coordinates: Coordinates,
    },
    MultiLineString {
        coordinates: Vec<Coordinates>,
    },
    Polygon {
        #[serde(deserialize_with = "deserialize_polygon")]
        coordinates: Coordinates,
    },
    MultiPolygon {
        coordinates: Vec<Vec<Coordinates>>,
    },
}

static mut PROCESSOR: u32 = 0;

struct JsonVisitor<'a, T> {
    processor: &'a mut u32,
    _type: PhantomData<fn() -> T>,
}

fn get_visitor<'a, T>() -> JsonVisitor<'a, T> {
    JsonVisitor {
        processor: unsafe { &mut PROCESSOR },
        _type: PhantomData,
    }
}

fn deserialize_properties<'de, D>(
    deserializer: D,
) -> Result<Map<String, serde_json::Value>, D::Error>
where
    D: Deserializer<'de>,
{
    impl<'de> Visitor<'de> for JsonVisitor<'_, Map<String, serde_json::Value>> {
        /// Return type of this visitor. This visitor computes the max of a
        /// sequence of values of type T, so the type of the maximum is T.
        type Value = Map<String, serde_json::Value>;

        fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
            formatter.write_str("a key value map")
        }

        fn visit_map<M>(self, mut access: M) -> Result<Self::Value, M::Error>
        where
            M: MapAccess<'de>,
        {
            dbg!("deserialize_properties");
            dbg!(self.processor);
            while let Some((key, value)) = access.next_entry::<String, serde_json::Value>()? {
                dbg!(key, value);
            }

            Ok(Map::new())
        }
    }

    let visitor = get_visitor::<Map<String, serde_json::Value>>();
    deserializer.deserialize_map(visitor)
}

const EMPTY_COORDINATES: Coordinates = Coordinates::new();

fn deserialize_polygon<'de, D>(deserializer: D) -> Result<Coordinates, D::Error>
where
    D: Deserializer<'de>,
{
    impl<'de> Visitor<'de> for JsonVisitor<'_, Coordinates> {
        type Value = Coordinates;

        fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
            formatter.write_str("a coordinate sequence")
        }

        fn visit_seq<S>(self, mut seq: S) -> Result<Coordinates, S::Error>
        where
            S: SeqAccess<'de>,
        {
            dbg!("deserialize_polygon");
            while let Some(coords) = seq.next_element::<Coordinates>()? {
                dbg!("ring");
                for coord in coords {
                    dbg!(coord);
                }
            }

            Ok(EMPTY_COORDINATES)
        }
    }

    let visitor = get_visitor::<Coordinates>();
    deserializer.deserialize_seq(visitor)
}

#[cfg(test)]
mod tests {
    use super::*;

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
    fn test_json_str() -> serde_json::Result<()> {
        let fc: FeatureCollection = serde_json::from_str(POLYGON)?;
        assert_eq!(fc.features.len(), 1);
        if let Geometry::Polygon { coordinates } = &fc.features[0].geometry {
            assert!(coordinates.is_empty());
        } else {
            assert!(false, "Geometry::Polygon expected");
        }
        assert!(false);
        Ok(())
    }

    #[test]
    fn test_missing_type() -> serde_json::Result<()> {
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
        Ok(())
    }

    #[test]
    fn test_from_reader() -> serde_json::Result<()> {
        let reader = std::io::Cursor::new(&POINT);
        let fc: FeatureCollection = serde_json::from_reader(reader)?;
        assert_eq!(fc.features.len(), 1);
        if let Geometry::Point { coordinates } = &fc.features[0].geometry {
            assert_eq!(coordinates, &(1.0, 1.0));
        } else {
            assert!(false, "Geometry::Point expected");
        }
        Ok(())
    }
}
