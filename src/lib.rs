#![allow(unused_must_use)]
use std::borrow::Cow;
use std::io::{Read, Seek};
use std::marker::PhantomData;
use std::string::FromUtf8Error;

use anyhow::Result;
use strong_xml::{XmlRead, XmlWrite};
pub use {roxmltree, strong_xml};

#[derive(Debug)]
pub struct Epub<R> {
    archive: zip::ZipArchive<R>,
}

impl<R: Read + Seek> Epub<R> {
    pub fn new(input: R) -> Result<Self> {
        let archive = zip::ZipArchive::new(input)?;
        let result = Self { archive };
        Ok(result)
    }

    pub fn read<Media>(&mut self, href: Href<'_, Media>) -> Result<Resource<Media>>
    where
        Media: media_type::MediaType,
        Media::Value: TryFrom<Vec<u8>>,
        <<Media as media_type::MediaType>::Value as TryFrom<Vec<u8>>>::Error: std::error::Error + Send + Sync + 'static,
    {
        let path = "OEBPS/".to_owned() + href.url.as_ref();
        let mut entry = self.archive.by_name(&path)?;
        let mut bytes = Vec::with_capacity(entry.size() as usize);
        entry.read_to_end(&mut bytes)?;

        Ok(Resource::new(bytes.try_into()?))
    }
}

#[derive(Debug, PartialEq, XmlWrite, XmlRead)]
#[xml(tag = "package")]
pub struct Content<'a> {
    #[xml(child = "metadata")]
    pub metadata: Metadata<'a>,
    #[xml(child = "manifest")]
    pub manifest: Manifest<'a>,
    #[xml(child = "spine")]
    pub spine: Spine<'a>,
    #[xml(child = "guide")]
    pub guide: Guide<'a>,
}

#[derive(Debug, PartialEq, XmlWrite, XmlRead)]
#[xml(tag = "metadata")]
pub struct Metadata<'a> {
    #[xml(flatten_text = "dc:title")]
    pub title: Cow<'a, str>,
    #[xml(flatten_text = "dc:language")]
    pub language: Cow<'a, str>,
    #[xml(flatten_text = "dc:identifier")]
    pub identifier: Cow<'a, str>,
}

#[derive(Debug, PartialEq, XmlWrite, XmlRead)]
#[xml(tag = "manifest")]
pub struct Manifest<'a> {
    #[xml(child = "item")]
    pub items: Vec<Item<'a>>,
}

#[derive(Debug, PartialEq, XmlWrite, XmlRead)]
#[xml(tag = "item")]
pub struct Item<'a> {
    #[xml(attr = "id")]
    pub id: Cow<'a, str>,
    #[xml(attr = "media-type")]
    pub media_type: Cow<'a, str>,
    #[xml(attr = "href")]
    href: Cow<'a, str>,
}

impl<'a> Item<'a> {
    pub fn xhtml_href(&'a self) -> Option<Href<'a, media_type::XHtml>> {
        self.match_href("application/xhtml+xml")
    }

    pub fn css_href(&'a self) -> Option<Href<'a, media_type::Css>> {
        self.match_href("text/css")
    }

    pub fn png_href(&'a self) -> Option<Href<'a, media_type::Png>> {
        self.match_href("image/png")
    }

    pub fn jpeg_href(&'a self) -> Option<Href<'a, media_type::Jpeg>> {
        self.match_href("image/jpeg")
    }

    pub fn gif_href(&'a self) -> Option<Href<'a, media_type::Gif>> {
        self.match_href("image/gif")
    }

    pub fn svg_href(&'a self) -> Option<Href<'a, media_type::Svg>> {
        self.match_href("image/svg+xml")
    }

    fn match_href<Media>(&'a self, media_type: &str) -> Option<Href<'a, Media>> {
        if self.media_type.as_ref() == media_type {
            Some(Href::new(self.href.clone()))
        } else {
            None
        }
    }
}

#[derive(Debug, PartialEq, XmlWrite, XmlRead)]
#[xml(tag = "spine")]
pub struct Spine<'a> {
    #[xml(child = "itemref")]
    pub refs: Vec<ItemRef<'a>>,
}

#[derive(Debug, PartialEq, XmlWrite, XmlRead)]
#[xml(tag = "itemref")]
pub struct ItemRef<'a> {
    #[xml(attr = "idref")]
    pub id_ref: Cow<'a, str>,
}

#[derive(Debug, PartialEq, XmlWrite, XmlRead)]
#[xml(tag = "guide")]
pub struct Guide<'a> {
    #[xml(child = "reference")]
    pub references: Vec<Reference<'a>>,
}

#[derive(Debug, PartialEq, XmlWrite, XmlRead)]
#[xml(tag = "reference")]
pub struct Reference<'a> {
    #[xml(attr = "type")]
    pub kind: Cow<'a, str>,
    #[xml(attr = "title")]
    pub title: Cow<'a, str>,
    #[xml(attr = "href")]
    href: Cow<'a, str>,
}

impl<'a> Reference<'a> {
    pub fn href(&'a self) -> Href<'a, media_type::XHtml> {
        Href::new(self.href.clone())
    }
}

#[derive(Debug, PartialEq, XmlWrite, XmlRead)]
#[xml(tag = "ncx")]
pub struct TableOfContents<'a> {
    #[xml(child = "navMap")]
    pub map: NavMap<'a>,
}

impl<'a> TableOfContents<'a> {
    pub fn points(&'a self) -> &'a [NavPoint<'a>] {
        &self.map.points
    }
}

#[derive(Debug, PartialEq, XmlWrite, XmlRead)]
#[xml(tag = "navMap")]
pub struct NavMap<'a> {
    #[xml(child = "navPoint")]
    pub points: Vec<NavPoint<'a>>,
}

#[derive(Debug, PartialEq, XmlWrite, XmlRead)]
#[xml(tag = "navPoint")]
pub struct NavPoint<'a> {
    #[xml(child = "navLabel")]
    pub label: NavLabel<'a>,
    #[xml(child = "content")]
    content: NavContent<'a>,
    #[xml(child = "navPoint")]
    pub children: Vec<Self>,
}

impl<'a> NavPoint<'a> {
    pub fn href(&'a self) -> Href<'a, media_type::XHtml> {
        Href::new(self.content.src.clone())
    }
}

#[derive(Debug, PartialEq, XmlWrite, XmlRead)]
#[xml(tag = "navLabel")]
pub struct NavLabel<'a> {
    #[xml(flatten_text = "text")]
    pub text: Cow<'a, str>,
}

#[derive(Debug, PartialEq, XmlWrite, XmlRead)]
#[xml(tag = "content")]
pub struct NavContent<'a> {
    #[xml(attr = "src")]
    src: Cow<'a, str>,
}

pub struct Resource<Media: media_type::MediaType> {
    pub data: Media::Value,
    phantom: PhantomData<Media>,
}

impl<Media: media_type::MediaType> Resource<Media> {
    fn new(data: Media::Value) -> Resource<Media> {
        Resource {
            data,
            phantom: PhantomData,
        }
    }
}

impl<'a> Resource<media_type::XHtml> {
    pub fn doc(&'a self) -> Result<roxmltree::Document> {
        Ok(roxmltree::Document::parse(&self.data.0)?)
    }
}

impl<'a> Resource<media_type::Opf> {
    pub fn content(&'a self) -> Result<Content> {
        Ok(Content::from_str(&self.data.0)?)
    }
}

impl<'a> Resource<media_type::DtbNcx> {
    pub fn toc(&'a self) -> Result<TableOfContents> {
        Ok(TableOfContents::from_str(&self.data.0)?)
    }
}

pub struct Href<'a, Media> {
    url: Cow<'a, str>,
    phantom: PhantomData<Media>,
}

impl Href<'static, media_type::DtbNcx> {
    pub const TOC: Self = Self::new(Cow::Borrowed("toc.ncx"));
}

impl Href<'static, media_type::Opf> {
    pub const CONTENT: Self = Self::new(Cow::Borrowed("content.opf"));
}

impl<'a, Media> Href<'a, Media> {
    const fn new(url: Cow<'a, str>) -> Self {
        Self {
            url,
            phantom: PhantomData,
        }
    }

    pub fn without_fragment(&'a self) -> Self {
        let url = self
            .url
            .split_once("#")
            .map(|(str, _)| Cow::Borrowed(str))
            .unwrap_or_else(|| self.url.clone());
        Self::new(url)
    }

    pub fn into_string(self) -> String {
        self.url.into_owned()
    }
}

impl<'a, Media> AsRef<str> for Href<'a, Media> {
    fn as_ref(&self) -> &str {
        self.url.as_ref()
    }
}

pub struct Utf8String(pub String);

impl TryFrom<Vec<u8>> for Utf8String {
    type Error = FromUtf8Error;

    fn try_from(value: Vec<u8>) -> Result<Self, Self::Error> {
        String::from_utf8(value).map(Utf8String)
    }
}

pub mod media_type {
    use super::Utf8String;

    pub struct Opf;
    pub struct DtbNcx;
    pub struct XHtml;
    pub struct Css;
    pub struct Png;
    pub struct Jpeg;
    pub struct Gif;
    pub struct Svg;

    pub trait MediaType {
        type Value;
    }
    impl MediaType for Opf {
        type Value = Utf8String;
    }
    impl MediaType for DtbNcx {
        type Value = Utf8String;
    }
    impl MediaType for XHtml {
        type Value = Utf8String;
    }
    impl MediaType for Css {
        type Value = Utf8String;
    }
    impl MediaType for Png {
        type Value = Vec<u8>;
    }
    impl MediaType for Jpeg {
        type Value = Vec<u8>;
    }
    impl MediaType for Gif {
        type Value = Vec<u8>;
    }
    impl MediaType for Svg {
        type Value = Vec<u8>;
    }
}
