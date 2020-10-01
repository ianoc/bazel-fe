use nom::bytes::complete::take_while1;
use nom::character::complete::digit1;
use nom::character::complete::line_ending;
use nom::error::ParseError;
use nom::multi::{many0, many1};
use nom::{bytes::complete::tag, combinator::map, combinator::opt, sequence::tuple, IResult};
use std::{collections::HashMap, error::Error};

pub struct IndexTable {
    tbl_map: HashMap<String, Vec<(u16, String)>>,
}
impl IndexTable {
    pub fn get<S>(&self, key: S) -> Option<&Vec<(u16, String)>>
    where
        S: Into<String>,
    {
        self.tbl_map.get(&key.into())
    }
}
fn element_extractor<'a, E>() -> impl Fn(&'a str) -> IResult<&str, (u16, &str), E>
where
    E: ParseError<&'a str>,
{
    map(
        tuple((
            digit1,
            tag(":"),
            take_while1(|chr| chr != ',' && chr != '\r' && chr != '\n'),
        )),
        |(freq, _, target)| {
            let f: &str = freq;
            (f.parse::<u16>().unwrap(), target)
        },
    )
}

fn parse_index_line<'a, E>() -> impl Fn(&'a str) -> IResult<&str, (String, Vec<(u16, String)>), E>
where
    E: ParseError<&'a str>,
{
    map(
        tuple((
            map(take_while1(|chr| chr != '\t'), |e: &str| e.to_string()),
            tag("\t"),
            many1(map(element_extractor(), |(freq, v)| (freq, v.to_string()))),
        )),
        |tup| (tup.0, tup.2),
    )
}

fn parse_file_e(input: &str) -> IResult<&str, Vec<(String, Vec<(u16, String)>)>> {
    many0(map(tuple((parse_index_line(), opt(line_ending))), |e| e.0))(input)
}

pub fn parse_file(input: &str) -> Result<IndexTable, Box<dyn Error>> {
    let extracted_result: Vec<(String, Vec<(u16, String)>)> = parse_file_e(input).unwrap().1;
    println!("Finished parsing..");

    let mut index_data = HashMap::new();
    index_data.reserve(extracted_result.len());

    for (k, v) in extracted_result.into_iter() {
        index_data.insert(k, v);
    }
    Ok(IndexTable {
        tbl_map: index_data,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    fn run_parse_index_line(input: &str) -> IResult<&str, (String, Vec<(u16, String)>)> {
        parse_index_line()(input)
    }

    #[test]
    fn parse_sample_line() {
        assert_eq!(
            run_parse_index_line(
                "PantsWorkaroundCache\t0:@third_party_jvm//3rdparty/jvm/com/twitter:util_cache"
            )
            .unwrap()
            .1,
            (
                String::from("PantsWorkaroundCache"),
                vec![(
                    0,
                    String::from("@third_party_jvm//3rdparty/jvm/com/twitter:util_cache")
                )]
            )
        );
    }

    #[test]
    fn parse_multiple_lines() {
        let parsed_file = parse_file(
            "scala.reflect.internal.SymbolPairs.Cursor.anon.1\t1:@third_party_jvm//3rdparty/jvm/org/scala_lang:scala_reflect
org.apache.parquet.thrift.test.TestPerson.TestPersonTupleScheme\t0:@third_party_jvm//3rdparty/jvm/org/apache/parquet:parquet_thrift_jar_tests
org.apache.commons.lang3.concurrent.MultiBackgroundInitializer\t68:@third_party_jvm//3rdparty/jvm/org/apache/commons:commons_lang3
org.apache.hadoop.util.GcTimeMonitor\t38:@third_party_jvm//3rdparty/jvm/org/apache/hadoop:hadoop_common
org.apache.hadoop.fs.FSProtos.FileStatusProto.FileType\t38:@third_party_jvm//3rdparty/jvm/org/apache/hadoop:hadoop_common
com.twitter.chill.JavaIterableWrapperSerializer\t2:@third_party_jvm//3rdparty/jvm/com/twitter:chill
org.apache.commons.collections4.map.ListOrderedMap$EntrySetView\t0:@third_party_jvm//3rdparty/jvm/org/apache/commons:commons_collections4
scala.collection.convert.AsJavaConverters\t41:@third_party_jvm//3rdparty/jvm/org/scala_lang:scala_library
org.ehcache.xml.XmlConfiguration.1\t0:@third_party_jvm//3rdparty/jvm/org/ehcache:ehcache
com.ibm.icu.text.CharsetRecog_sbcs$CharsetRecog_8859_1_de\t1:@third_party_jvm//3rdparty/jvm/com/ibm/icu:icu4j
scala.reflect.internal.Definitions$DefinitionsClass$VarArityClass\t1:@third_party_jvm//3rdparty/jvm/org/scala_lang:scala_reflect
org.apache.http.nio.pool.AbstractNIOConnPool.1\t0:@third_party_jvm//3rdparty/jvm/org/apache/httpcomponents:httpcore_nio
io.circe.generic.util.macros.DerivationMacros$$typecreator1$1 21:@third_party_jvm//3rdparty/jvm/io/circe:circe_generic
org.apache.zookeeper.server.NettyServerCnxn.DumpCommand\t0:@third_party_jvm//3rdparty/jvm/org/apache/zookeeper:zookeeper
org.apache.logging.log4j.core.appender.OutputStreamAppender$OutputStreamManagerFactory\t53:@third_party_jvm//3rdparty/jvm/org/apache/logging/log4j:log4j_core
com.twitter.finagle.http.service.RoutingService.anonfun\t2:@third_party_jvm//3rdparty/jvm/com/twitter:finagle_http
org.bouncycastle.util.CollectionStor\t10:@third_party_jvm//3rdparty/jvm/org/bouncycastle:bcprov_jdk15on
org.apache.avro.io.parsing.JsonGrammarGenerator$1\t0:@third_party_jvm//3rdparty/jvm/org/apache/avro:avro
org.terracotta.statistics.util\t0:@third_party_jvm//3rdparty/jvm/org/ehcache:ehcache
com.ibm.icu.impl.Normalizer2Impl$1\t1:@third_party_jvm//3rdparty/jvm/com/ibm/icu:icu4j
org.eclipse.jetty.io.ByteBufferPool.Bucket\t0:@third_party_jvm//3rdparty/jvm/org/eclipse"
        ).unwrap();

        assert_eq!(
            parsed_file.get("org.apache.parquet.thrift.test.TestPerson.TestPersonTupleScheme"),
            Some(&vec![(
                0,
                String::from(
                    "@third_party_jvm//3rdparty/jvm/org/apache/parquet:parquet_thrift_jar_tests"
                )
            )])
        );
    }
}
