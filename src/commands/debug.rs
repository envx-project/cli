use super::*;
use pgp::{composed::message::Message, Deserializable};

/// Unset the current project
#[derive(Parser)]
pub struct Args {}

pub async fn command(_args: Args, _json: bool) -> Result<()> {
    let msg = "-----BEGIN PGP MESSAGE-----

wcBMA0H2mhUYgmEfAQgAjbXmEmF19vXJAnJsabQRYOpC3VPSe3EscdDMUrpyMM/o
0G3rvVbxfybm8cNbKvghUyY03DBTENAwBe4HNY+fOuL9ueRciPOXerKZhOVl0vLn
rfJ8xZqi0PpVIXerGAVYX1h6YflupPgWSeYK8oci+dLNjTdz+EqaM4FiQi1zgxdS
ceNsTKRYSv29rQgbDpIWFhPZ7TyV3U/4S2pyFkkZIbfAZIjuLEIv80dsmHzFRhru
UB4MG3RQFw65SfBMW5evp62/5PMymIWqven20ECyTZHdu148PPhKoobjLNA6ujHW
rAVCaMclFJ3dtB5tGocDxazs23aNowaKppa3UdYOh8HATAPJ2p28Gm1WhAEH/i/6
0pw10h6i6utABZRvdHeGtcE2/pnHBWPeT2fRf+q+2X2uD2iXUJdHN0akrH6HYeeK
XKLZqHq+iXh3xZEabUDQ0RStvIYVMD61mzmCDcsSF1rZJimKhDFRQMyCfCGI3F5v
TZENZ2PFzYNm4Cr3ha7CV1XSUzlTlamWzJWx1shqLdWAth1KN+kNQVUGxrGNZnle
g0tDZh30hlsQPU57gMxrn0tR+BEnqrhGLxCYq600Jwn2bfNdFESvEniLrqDpmAET
7PthtSy3xq5USpTHJC+iED8swm8/w5WIn9tUSz+hdlqlkN4GzW8lXeRA2YIF6plr
6hPJZJ9LJCv2RknRysHBwEwDYcxVrtlYS3gBCADKOmDwymTTVOwPm871OcEFNI0r
7Vy25XDLAX8f8rsegSD9fwuGSag1wMB9uWpcsq1aBaWGH//ygv6tEEk/pYFBWUJz
u7OTHKPKAmRUFpmTRu06EmrUSYVvYqJuQQFtm5iWNTjADgN0mT8jzsax7iV7zFW2
M238kcrebEmbVMshovqZYxpsLRzIs/oqf81sA7L1cSJoO/XSqLujOMwhPAuMEdlt
QOZJGznjKDExvm2OOz5Ldm62wq4f5g7AGJ2Ru9JNlNDbv2vL0ytWyzT3Issa4DGI
RMiYd9WjHTzYkYbUdywD2+NsaelAYkv5zwxmbDrnjFwKJkLGiz3kNqItemFb0mgB
cTIl3kY9gWRj1tipPkQDljpyHAP5Gp/ylJooSnvPpThsjtJ50YIF2Ybk9pn+ZUcE
DRqiBUlH1H97H7fyMzw1dXAP5YoR7Ztq/al63MukPJCGZp8Zm36Qu26lQU69XZwf
oqXSSlm+IQ==
=aaLV
-----END PGP MESSAGE-----
";

    let msg = Message::from_string(msg)?;

    msg.0.get_recipients().iter().for_each(|r| {
        println!("{:?}", r);
    });

    Ok(())
}
