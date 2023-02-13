//! TODO Homolog of `aiochris.Search`
//! https://github.com/FNNDSC/aiochris/blob/adaff5bbc1d4d886ec2ca8155d82d266fa81d093/chris/util/search.py
pub struct Search<'a> {
    client: reqwest::Client,
    base_url: &'a str,
}
