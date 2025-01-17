use itertools::Itertools;

/// `Sitemap` stores the sitemap for the fpm package defines in the FPM.ftd
///
/// ```ftd
/// -- fpm.sitemap:
///
/// # foo/
/// ## bar/
/// - doc-1/
///   - childdoc-1/
/// - doc-2/
/// ```
///
/// In above example, the id starts with `#` becomes the section. Similarly the id
/// starts with `##` becomes the subsection and then the id starts with `-` becomes
/// the table od content (TOC).
#[derive(Debug, Clone, Default)]
pub struct Sitemap {
    pub sections: Vec<Section>,
}

#[derive(Debug, Clone, Default)]
pub struct Section {
    /// `id` is the document id (or url) provided in the section
    /// Example:
    ///
    /// ```ftd
    ///
    /// # foo/
    ///
    /// ```
    ///
    /// Here foo/ is store as `id`
    pub id: String,

    /// `title` contains the title of the document. This can be specified inside
    /// document itself.
    ///
    /// Example: In the foo.ftd document
    ///
    /// ```ftd
    /// -- fpm.info DOCUMENT_INFO:
    /// title: Foo Title
    /// ```
    ///
    /// In above example the `title` stores `Foo Title`.
    ///
    /// In the case where the title is not defined as above, the title would be
    /// according to heading priority
    ///
    /// Example: In the foo.ftd document
    ///
    /// ```ftd
    ///
    /// -- ft.h0: Foo Heading Title
    /// ```
    /// In above example, the `title` stores `Foo Heading Title`.
    pub title: Option<String>,

    /// `file_location` stores the location of the document in the
    /// file system
    ///
    /// In case of translation package, it stores the location in original
    /// package
    /// It is an optional field as the id provided could be an url to a website.
    /// Eg:
    /// ```ftd
    /// # Fifthtry: https://fifthtry.com/
    /// ````
    /// In that case it store `None`
    pub file_location: Option<camino::Utf8PathBuf>,

    /// `translation_file_location` has value in case of translation package.
    /// It stores the location of the document in the
    /// file system in the translation package.
    pub translation_file_location: Option<camino::Utf8PathBuf>,

    /// `extra_data` stores the key value data provided in the section.
    /// This is passed as context and consumes by processors like `get-data`.
    ///
    /// Example:
    ///
    /// In `FPM.ftd`
    ///
    /// ```fpm
    /// -- fpm.sitemap:
    ///
    /// \# foo/
    /// show: true
    /// message: Hello World
    /// ```
    ///
    /// In `foo.ftd`
    ///
    /// ```ftd
    ///
    /// -- boolean show:
    /// $processor$: get-data
    ///
    /// -- string message:
    /// $processor$: get-data
    /// ```
    ///
    /// The above example injects the value `true` and `Hello World`
    /// to the variables `show` and `message` respectively in foo.ftd
    /// and then renders it.
    pub extra_data: std::collections::BTreeMap<String, String>,
    pub is_active: bool,
    pub nav_title: Option<String>,
    pub subsections: Vec<Subsection>,
}

#[derive(Debug, Clone)]
pub struct Subsection {
    pub id: Option<String>,
    pub title: Option<String>,
    pub file_location: Option<camino::Utf8PathBuf>,
    pub translation_file_location: Option<camino::Utf8PathBuf>,
    pub visible: bool,
    pub extra_data: std::collections::BTreeMap<String, String>,
    pub is_active: bool,
    pub nav_title: Option<String>,
    pub toc: Vec<TocItem>,
}

impl Default for Subsection {
    fn default() -> Self {
        Subsection {
            id: None,
            title: None,
            file_location: Default::default(),
            translation_file_location: None,
            visible: true,
            extra_data: Default::default(),
            is_active: false,
            nav_title: None,
            toc: vec![],
        }
    }
}

#[derive(Debug, Clone, Default)]
pub struct TocItem {
    pub id: String,
    pub title: Option<String>,
    pub file_location: Option<camino::Utf8PathBuf>,
    pub translation_file_location: Option<camino::Utf8PathBuf>,
    pub extra_data: std::collections::BTreeMap<String, String>,
    pub is_active: bool,
    pub nav_title: Option<String>,
    pub children: Vec<TocItem>,
}

#[derive(Debug, Default, serde::Serialize)]
pub struct SiteMapCompat {
    pub sections: Vec<TocItemCompat>,
    pub subsections: Vec<TocItemCompat>,
    pub toc: Vec<TocItemCompat>,
    #[serde(rename = "current-section")]
    pub current_section: Option<TocItemCompat>,
    #[serde(rename = "current-subsection")]
    pub current_subsection: Option<TocItemCompat>,
    #[serde(rename = "current-page")]
    pub current_page: Option<TocItemCompat>,
}

#[derive(Debug, Default, Clone, serde::Serialize)]
pub struct TocItemCompat {
    pub url: Option<String>,
    pub number: Option<String>,
    pub title: Option<String>,
    #[serde(rename = "is-heading")]
    pub is_heading: bool,
    // TODO: Font icon mapping to html?
    #[serde(rename = "font-icon")]
    pub font_icon: Option<String>,
    #[serde(rename = "is-disabled")]
    pub is_disabled: bool,
    #[serde(rename = "is-active")]
    pub is_active: bool,
    #[serde(rename = "is-open")]
    pub is_open: bool,
    #[serde(rename = "img-src")]
    pub image_src: Option<String>,
    pub children: Vec<TocItemCompat>,
}

impl TocItemCompat {
    fn new(
        url: Option<String>,
        title: Option<String>,
        is_active: bool,
        is_open: bool,
    ) -> TocItemCompat {
        TocItemCompat {
            url,
            number: None,
            title,
            is_heading: false,
            font_icon: None,
            is_disabled: false,
            is_active,
            is_open,
            image_src: None,
            children: vec![],
        }
    }
}

#[derive(Debug, Clone)]
pub enum SitemapElement {
    Section(Section),
    Subsection(Subsection),
    TocItem(TocItem),
}

impl SitemapElement {
    pub(crate) fn insert_key_value(&mut self, key: &str, value: &str) {
        let element_title = match self {
            SitemapElement::Section(s) => &mut s.extra_data,
            SitemapElement::Subsection(s) => &mut s.extra_data,
            SitemapElement::TocItem(s) => &mut s.extra_data,
        };
        element_title.insert(key.to_string(), value.trim().to_string());
    }

    pub(crate) fn set_title(&mut self, title: Option<String>) {
        let element_title = match self {
            SitemapElement::Section(s) => &mut s.title,
            SitemapElement::Subsection(s) => &mut s.title,
            SitemapElement::TocItem(s) => &mut s.title,
        };
        *element_title = title;
    }

    pub(crate) fn set_id(&mut self, id: Option<String>) {
        let id = if let Some(id) = id {
            id
        } else {
            return;
        };
        match self {
            SitemapElement::Section(s) => {
                s.id = id;
            }
            SitemapElement::Subsection(s) => {
                s.id = Some(id);
            }
            SitemapElement::TocItem(s) => {
                s.id = id;
            }
        };
    }

    pub(crate) fn set_nav_title(&mut self, nav_title: Option<String>) {
        let nav = match self {
            SitemapElement::Section(s) => &mut s.nav_title,
            SitemapElement::Subsection(s) => &mut s.nav_title,
            SitemapElement::TocItem(s) => &mut s.nav_title,
        };
        *nav = nav_title;
    }

    pub(crate) fn get_title(&self) -> Option<String> {
        match self {
            SitemapElement::Section(s) => &s.title,
            SitemapElement::Subsection(s) => &s.title,
            SitemapElement::TocItem(s) => &s.title,
        }
        .clone()
    }

    pub(crate) fn get_id(&self) -> Option<String> {
        match self {
            SitemapElement::Section(s) => Some(s.id.clone()),
            SitemapElement::Subsection(s) => s.id.clone(),
            SitemapElement::TocItem(s) => Some(s.id.clone()),
        }
    }
}

#[derive(thiserror::Error, Debug)]
pub enum ParseError {
    #[error("{doc_id} -> {message} -> Row Content: {row_content}")]
    InvalidTOCItem {
        doc_id: String,
        message: String,
        row_content: String,
    },
}

#[derive(Debug, Clone, PartialEq)]
enum ParsingState {
    WaitingForSection,
    ParsingSection,
    ParsingSubsection,
    ParsingTOC,
}
#[derive(Debug)]
pub struct SitemapParser {
    state: ParsingState,
    sections: Vec<(SitemapElement, usize)>,
    temp_item: Option<(SitemapElement, usize)>,
    doc_name: String,
}

impl SitemapParser {
    pub fn read_line(&mut self, line: &str) -> Result<(), ParseError> {
        // The row could be one of the 4 things:

        // - Heading
        // - Prefix/suffix item
        // - Separator
        // - ToC item
        if line.trim().is_empty() {
            return Ok(());
        }
        let mut iter = line.chars();
        let mut depth = 0;
        let mut rest = "".to_string();
        loop {
            match iter.next() {
                Some(' ') => {
                    depth += 1;
                    iter.next();
                }
                Some('-') => {
                    rest = iter.collect::<String>();
                    if ![
                        ParsingState::ParsingSection,
                        ParsingState::ParsingSubsection,
                        ParsingState::ParsingTOC,
                    ]
                    .contains(&self.state)
                    {
                        return Err(ParseError::InvalidTOCItem {
                            doc_id: self.doc_name.clone(),
                            message: "Ambiguous <title>: <URL> evaluation. TOC is found before section or subsection".to_string(),
                            row_content: rest.as_str().to_string(),
                        });
                    }
                    self.state = ParsingState::ParsingTOC;
                    break;
                }
                Some('#') => {
                    // Heading can not have any attributes. Append the item and look for the next input
                    rest = iter.collect::<String>();
                    self.state = ParsingState::ParsingSection;
                    if let Some(content) = rest.strip_prefix('#') {
                        if !ParsingState::ParsingSection.eq(&self.state) {
                            return Err(ParseError::InvalidTOCItem {
                                doc_id: self.doc_name.clone(),
                                message: "Ambiguous <title>: <URL> evaluation. Subsection is called before subsection".to_string(),
                                row_content: rest.as_str().to_string(),
                            });
                        }
                        rest = content.to_string();
                        self.state = ParsingState::ParsingSubsection;
                    }
                    break;
                }
                Some(k) => {
                    let l = format!("{}{}", k, iter.collect::<String>());
                    self.read_attrs(l.as_str())?;
                    return Ok(());
                    // panic!()
                }
                None => {
                    break;
                }
            }
        }
        self.eval_temp_item()?;

        // Stop eager checking, Instead of split and evaluate URL/title, first push
        // The complete string, postprocess if url doesn't exist
        let sitemapelement = match self.state {
            ParsingState::WaitingForSection => SitemapElement::Section(Section {
                id: rest.as_str().trim().to_string(),
                ..Default::default()
            }),
            ParsingState::ParsingSection => SitemapElement::Section(Section {
                id: rest.as_str().trim().to_string(),
                ..Default::default()
            }),
            ParsingState::ParsingSubsection => SitemapElement::Subsection(Subsection {
                id: Some(rest.as_str().trim().to_string()),
                ..Default::default()
            }),
            ParsingState::ParsingTOC => SitemapElement::TocItem(TocItem {
                id: rest.as_str().trim().to_string(),
                ..Default::default()
            }),
        };
        self.temp_item = Some((sitemapelement, depth));
        Ok(())
    }

    fn eval_temp_item(&mut self) -> Result<(), ParseError> {
        if let Some((ref toc_item, depth)) = self.temp_item {
            // Split the line by `:`. title = 0, url = Option<1>
            let resp_item = if toc_item.get_title().is_none() && toc_item.get_id().is_some() {
                // URL not defined, Try splitting the title to evaluate the URL
                let current_title = toc_item.get_id().unwrap();
                let (title, url) = match current_title.as_str().matches(':').count() {
                    1 | 0 => {
                        if let Some((first, second)) = current_title.rsplit_once(":") {
                            (
                                Some(first.trim().to_string()),
                                Some(second.trim().to_string()),
                            )
                        } else {
                            // No matches, i.e. return the current string as title, url as none
                            (Some(current_title), None)
                        }
                    }
                    _ => {
                        // The URL can have its own colons. So match the URL first
                        let url_regex = fpm::utils::url_regex();
                        if let Some(regex_match) = url_regex.find(current_title.as_str()) {
                            let curr_title = current_title.as_str();
                            (
                                Some(curr_title[..regex_match.start()].trim().to_string()),
                                Some(
                                    curr_title[regex_match.start()..regex_match.end()]
                                        .trim_start_matches(':')
                                        .trim()
                                        .to_string(),
                                ),
                            )
                        } else {
                            return Err(ParseError::InvalidTOCItem {
                                doc_id: self.doc_name.clone(),
                                message: "Ambiguous <title>: <URL> evaluation. Multiple colons found. Either specify the complete URL or specify the url as an attribute".to_string(),
                                row_content: current_title.as_str().to_string(),
                            });
                        }
                    }
                };

                {
                    let mut toc_item = toc_item.clone();
                    toc_item.set_id(url);
                    toc_item.set_title(title);
                    toc_item
                }
            } else {
                let id = toc_item.get_id();
                let mut toc_item = toc_item.clone();
                toc_item.set_id(id);
                toc_item
            };
            self.sections.push((resp_item, depth))
        }
        self.temp_item = None;
        Ok(())
    }
    fn read_attrs(&mut self, line: &str) -> Result<(), ParseError> {
        if line.trim().is_empty() {
            // Empty line found. Process the temp_item
            self.eval_temp_item()?;
        } else {
            // let id = self.temp_item.unwrap().0.get_id();
            match &mut self.temp_item {
                Some((i, _)) => match line.split_once(":") {
                    Some((k, v)) => {
                        let v = v.trim();
                        let id = i.get_id();
                        if k.eq("url") {
                            i.set_id(Some(v.to_string()));
                            if i.get_title().is_none() {
                                i.set_title(id);
                            }
                        }
                        if k.eq("nav-title") {
                            i.set_nav_title(Some(v.to_string()));
                        }
                        i.insert_key_value(k, v);
                    }
                    _ => todo!(),
                },
                _ => panic!("State mismatch"),
            };
        };
        Ok(())
    }

    fn finalize(self) -> Result<Vec<(SitemapElement, usize)>, ParseError> {
        Ok(self.sections)
    }
}

impl Sitemap {
    pub fn parse(
        s: &str,
        package: &fpm::Package,
        config: &fpm::Config,
        asset_documents: &std::collections::HashMap<String, String>,
        base_url: &str,
    ) -> Result<Self, ParseError> {
        let mut parser = SitemapParser {
            state: ParsingState::WaitingForSection,
            sections: vec![],
            temp_item: None,
            doc_name: package.name.to_string(),
        };
        for line in s.split('\n') {
            parser.read_line(line)?;
        }
        if parser.temp_item.is_some() {
            parser.eval_temp_item()?;
        }
        let mut sitemap = Sitemap {
            sections: construct_tree_util(parser.finalize()?),
        };

        sitemap
            .resolve(package, config, asset_documents, base_url)
            .map_err(|e| ParseError::InvalidTOCItem {
                doc_id: package.name.to_string(),
                message: e.to_string(),
                row_content: "".to_string(),
            })?;

        Ok(sitemap)
    }

    fn resolve(
        &mut self,
        package: &fpm::Package,
        config: &fpm::Config,
        asset_documents: &std::collections::HashMap<String, String>,
        base_url: &str,
    ) -> fpm::Result<()> {
        let package_root = config.get_root_for_package(package);
        let current_package_root = config.root.to_owned();
        for section in self.sections.iter_mut() {
            resolve_section(
                section,
                &package_root,
                &current_package_root,
                asset_documents,
                base_url,
                config,
            )?;
        }
        return Ok(());

        fn resolve_section(
            section: &mut fpm::sitemap::Section,
            package_root: &camino::Utf8PathBuf,
            current_package_root: &camino::Utf8PathBuf,
            asset_documents: &std::collections::HashMap<String, String>,
            base_url: &str,
            config: &fpm::Config,
        ) -> fpm::Result<()> {
            let (file_location, translation_file_location) =
                if fpm::utils::url_regex().find(section.id.as_str()).is_some() {
                    (None, None)
                } else {
                    match fpm::Config::get_file_name(current_package_root, section.id.as_str()) {
                        Ok(name) => {
                            if current_package_root.eq(package_root) {
                                (Some(current_package_root.join(name)), None)
                            } else {
                                (
                                    Some(package_root.join(name.as_str())),
                                    Some(current_package_root.join(name)),
                                )
                            }
                        }
                        Err(_) => (
                            Some(
                                package_root.join(
                                    fpm::Config::get_file_name(package_root, section.id.as_str())
                                        .map_err(|e| fpm::Error::UsageError {
                                        message: format!(
                                        "`{}` not found, fix fpm.sitemap in FPM.ftd. Error: {:?}",
                                        section.id, e
                                    ),
                                    })?,
                                ),
                            ),
                            None,
                        ),
                    }
                };
            section.file_location = file_location;
            section.translation_file_location = translation_file_location;

            for subsection in section.subsections.iter_mut() {
                resolve_subsection(
                    subsection,
                    package_root,
                    current_package_root,
                    asset_documents,
                    base_url,
                    config,
                )?;
            }
            Ok(())
        }

        fn resolve_subsection(
            subsection: &mut fpm::sitemap::Subsection,
            package_root: &camino::Utf8PathBuf,
            current_package_root: &camino::Utf8PathBuf,
            asset_documents: &std::collections::HashMap<String, String>,
            base_url: &str,
            config: &fpm::Config,
        ) -> fpm::Result<()> {
            if let Some(ref id) = subsection.id {
                let (file_location, translation_file_location) = if fpm::utils::url_regex()
                    .find(id.as_str())
                    .is_some()
                {
                    (None, None)
                } else {
                    match fpm::Config::get_file_name(current_package_root, id.as_str()) {
                            Ok(name) => {
                                if current_package_root.eq(package_root) {
                                    (Some(current_package_root.join(name)), None)
                                } else {
                                    (
                                        Some(package_root.join(name.as_str())),
                                        Some(current_package_root.join(name)),
                                    )
                                }
                            }
                            Err(_) => (
                                Some(package_root.join(
                                    fpm::Config::get_file_name(package_root, id.as_str()).map_err(
                                        |e| fpm::Error::UsageError {
                                            message: format!(
                                                "`{}` not found, fix fpm.sitemap in FPM.ftd. Error: {:?}",
                                                id, e
                                            ),
                                        },
                                    )?,
                                )),
                                None,
                            ),
                        }
                };
                subsection.file_location = file_location;
                subsection.translation_file_location = translation_file_location;
            }

            for toc in subsection.toc.iter_mut() {
                resolve_toc(
                    toc,
                    package_root,
                    current_package_root,
                    asset_documents,
                    base_url,
                    config,
                )?;
            }
            Ok(())
        }

        fn resolve_toc(
            toc: &mut fpm::sitemap::TocItem,
            package_root: &camino::Utf8PathBuf,
            current_package_root: &camino::Utf8PathBuf,
            asset_documents: &std::collections::HashMap<String, String>,
            base_url: &str,
            config: &fpm::Config,
        ) -> fpm::Result<()> {
            let (file_location, translation_file_location) = if toc.id.trim().is_empty()
                || fpm::utils::url_regex().find(toc.id.as_str()).is_some()
            {
                (None, None)
            } else {
                match fpm::Config::get_file_name(current_package_root, toc.id.as_str()) {
                    Ok(name) => {
                        if current_package_root.eq(package_root) {
                            (Some(current_package_root.join(name)), None)
                        } else {
                            (
                                Some(package_root.join(name.as_str())),
                                Some(current_package_root.join(name)),
                            )
                        }
                    }
                    Err(_) => (
                        Some(package_root.join(
                            fpm::Config::get_file_name(package_root, toc.id.as_str()).map_err(
                                |e| fpm::Error::UsageError {
                                    message: format!(
                                        "`{}` not found, fix fpm.sitemap in FPM.ftd. Error: {:?}",
                                        toc.id, e
                                    ),
                                },
                            )?,
                        )),
                        None,
                    ),
                }
            };
            toc.file_location = file_location;
            toc.translation_file_location = translation_file_location;

            for toc in toc.children.iter_mut() {
                resolve_toc(
                    toc,
                    package_root,
                    current_package_root,
                    asset_documents,
                    base_url,
                    config,
                )?;
            }
            Ok(())
        }
    }

    /// `get_all_locations` returns the list of tuple containing the following values:
    /// (
    ///     file_location: &camino::Utf8PathBuf, // The location of the document in the file system.
    ///                     In case of translation package, the location in the original package
    ///     translation_file_location: &Option<camino::Utf8PathBuf> // In case of the translation package,
    ///                         The location of the document in the current/translation package
    ///     url: &Option<String> // expected url for the document.
    /// )
    pub(crate) fn get_all_locations(
        &self,
    ) -> Vec<(
        &camino::Utf8PathBuf,
        &Option<camino::Utf8PathBuf>,
        Option<String>,
    )> {
        let mut locations = vec![];
        for section in self.sections.iter() {
            if let Some(ref file_location) = section.file_location {
                locations.push((
                    file_location,
                    &section.translation_file_location,
                    get_id(section.id.as_str()),
                ));
            }
            for subsection in section.subsections.iter() {
                if subsection.visible {
                    if let Some(ref file_location) = subsection.file_location {
                        locations.push((
                            file_location,
                            &subsection.translation_file_location,
                            subsection.id.as_ref().map(|v| get_id(v.as_str())).flatten(),
                        ));
                    }
                }
                for toc in subsection.toc.iter() {
                    if let Some(ref file_location) = toc.file_location {
                        locations.push((
                            file_location,
                            &toc.translation_file_location,
                            get_id(toc.id.as_str()),
                        ));
                    }
                    locations.extend(get_toc_locations(toc));
                }
            }
        }
        return locations;

        fn get_id(id: &str) -> Option<String> {
            if id.contains("-/") {
                return Some(id.to_string());
            }
            None
        }

        fn get_toc_locations(
            toc: &fpm::sitemap::TocItem,
        ) -> Vec<(
            &camino::Utf8PathBuf,
            &Option<camino::Utf8PathBuf>,
            Option<String>,
        )> {
            let mut locations = vec![];
            for child in toc.children.iter() {
                if let Some(ref file_location) = child.file_location {
                    locations.push((
                        file_location,
                        &child.translation_file_location,
                        get_id(child.id.as_str()),
                    ));
                }
                locations.extend(get_toc_locations(child));
            }
            locations
        }
    }

    fn ids_matches(id1: &str, id2: &str) -> bool {
        return strip_id(id1).eq(&strip_id(id2));

        fn strip_id(id: &str) -> String {
            let id = id
                .trim()
                .replace("/index.html", "/")
                .replace("index.html", "/");
            if id.eq("/") {
                return id;
            }
            id.trim_matches('/').to_string()
        }
    }

    pub(crate) fn get_sitemap_by_id(&self, id: &str) -> Option<SiteMapCompat> {
        let mut sections = vec![];
        let mut subsections = vec![];
        let mut toc = vec![];
        let mut index = 0;
        let mut current_section = None;
        let mut current_subsection = None;
        let mut current_page = None;
        for (idx, section) in self.sections.iter().enumerate() {
            index = idx;

            if fpm::sitemap::Sitemap::ids_matches(section.id.as_str(), id) {
                subsections = section
                    .subsections
                    .iter()
                    .filter(|v| v.visible)
                    .map(|v| {
                        let active =
                            v.id.as_ref()
                                .map(|v| fpm::sitemap::Sitemap::ids_matches(v, id))
                                .unwrap_or(false);
                        let toc = TocItemCompat::new(v.id.clone(), v.title.clone(), active, active);
                        if active {
                            let mut curr_subsection = toc.clone();
                            if let Some(ref title) = v.nav_title {
                                curr_subsection.title = Some(title.to_string());
                            }
                            current_subsection = Some(curr_subsection);
                        }
                        toc
                    })
                    .collect();

                if let Some(sub) = section
                    .subsections
                    .iter()
                    .find_or_first(|v| {
                        v.id.as_ref()
                            .map(|v| fpm::sitemap::Sitemap::ids_matches(v, id))
                            .unwrap_or(false)
                    })
                    .or_else(|| section.subsections.first())
                {
                    let (toc_list, current_toc) = get_all_toc(sub.toc.as_slice(), id);
                    toc.extend(toc_list);
                    current_page = current_toc;
                }
                let mut section_toc = TocItemCompat::new(
                    Some(get_url(section.id.as_str())),
                    section.title.clone(),
                    true,
                    true,
                );
                sections.push(section_toc.clone());
                if let Some(ref title) = section.nav_title {
                    section_toc.title = Some(title.to_string());
                }
                current_section = Some(section_toc);
                break;
            }

            if let Some((subsection_list, toc_list, curr_subsection, curr_toc)) =
                get_subsection_by_id(id, section.subsections.as_slice())
            {
                subsections.extend(subsection_list);
                toc.extend(toc_list);
                current_subsection = curr_subsection;
                current_page = curr_toc;
                let mut section_toc = TocItemCompat::new(
                    Some(get_url(section.id.as_str())),
                    section.title.clone(),
                    true,
                    true,
                );
                sections.push(section_toc.clone());
                if let Some(ref title) = section.nav_title {
                    section_toc.title = Some(title.to_string());
                }
                current_section = Some(section_toc);
                break;
            }

            sections.push(TocItemCompat::new(
                Some(get_url(section.id.as_str())),
                section.title.clone(),
                false,
                false,
            ));
        }
        sections.extend(self.sections[index + 1..].iter().map(|v| {
            TocItemCompat::new(Some(get_url(v.id.as_str())), v.title.clone(), false, false)
        }));
        return Some(SiteMapCompat {
            sections,
            subsections,
            toc,
            current_section,
            current_subsection,
            current_page,
        });

        #[allow(clippy::type_complexity)]
        fn get_subsection_by_id(
            id: &str,
            subsections: &[Subsection],
        ) -> Option<(
            Vec<TocItemCompat>,
            Vec<TocItemCompat>,
            Option<TocItemCompat>,
            Option<TocItemCompat>,
        )> {
            let mut subsection_list = vec![];
            let mut toc = vec![];
            let mut index = 0;
            let mut found = false;
            let mut current_subsection = None;
            let mut current_page = None;

            for (idx, subsection) in subsections.iter().enumerate() {
                index = idx;
                if subsection.visible
                    && subsection
                        .id
                        .as_ref()
                        .map(|v| fpm::sitemap::Sitemap::ids_matches(v, id))
                        .unwrap_or(false)
                {
                    let (toc_list, current_toc) = get_all_toc(subsection.toc.as_slice(), id);
                    toc.extend(toc_list);
                    current_page = current_toc;
                    let mut subsection_toc = TocItemCompat::new(
                        subsection.id.as_ref().map(|v| get_url(v.as_str())),
                        subsection.title.clone(),
                        true,
                        true,
                    );
                    subsection_list.push(subsection_toc.clone());
                    if let Some(ref title) = subsection.nav_title {
                        subsection_toc.title = Some(title.to_string());
                    }
                    current_subsection = Some(subsection_toc);
                    found = true;
                    break;
                }

                if let Some((toc_list, current_toc)) = get_toc_by_id(id, subsection.toc.as_slice())
                {
                    toc.extend(toc_list);
                    current_page = Some(current_toc);
                    if subsection.visible {
                        let mut subsection_toc = TocItemCompat::new(
                            subsection.id.as_ref().map(|v| get_url(v.as_str())),
                            subsection.title.clone(),
                            true,
                            true,
                        );
                        subsection_list.push(subsection_toc.clone());
                        if let Some(ref title) = subsection.nav_title {
                            subsection_toc.title = Some(title.to_string());
                        }
                        current_subsection = Some(subsection_toc);
                    }
                    found = true;
                    break;
                }

                subsection_list.push(TocItemCompat::new(
                    subsection.id.as_ref().map(|v| get_url(v.as_str())),
                    subsection.title.clone(),
                    false,
                    false,
                ));
            }

            if found {
                subsection_list.extend(
                    subsections[index + 1..]
                        .iter()
                        .map(|v| TocItemCompat::new(v.id.clone(), v.title.clone(), false, false)),
                );
                return Some((subsection_list, toc, current_subsection, current_page));
            }
            None
        }

        fn get_all_toc(toc: &[TocItem], id: &str) -> (Vec<TocItemCompat>, Option<TocItemCompat>) {
            let mut current_page = None;
            let toc = get_toc_by_id_(id, toc, &mut current_page).1;
            (toc, current_page)
        }

        fn get_toc_by_id(id: &str, toc: &[TocItem]) -> Option<(Vec<TocItemCompat>, TocItemCompat)> {
            let mut current_page = None;
            let toc_list = get_toc_by_id_(id, toc, &mut current_page).1;
            if let Some(current_page) = current_page {
                return Some((toc_list, current_page));
            }
            None
        }

        fn get_toc_by_id_(
            id: &str,
            toc: &[TocItem],
            current_page: &mut Option<TocItemCompat>,
        ) -> (bool, Vec<TocItemCompat>) {
            let mut toc_list = vec![];
            let mut found_here = false;

            for toc_item in toc.iter() {
                let mut current_toc = {
                    let (is_open, children) =
                        get_toc_by_id_(id, toc_item.children.as_slice(), current_page);
                    let is_active = fpm::sitemap::Sitemap::ids_matches(toc_item.id.as_str(), id);
                    let mut current_toc = TocItemCompat::new(
                        Some(get_url(toc_item.id.as_str()).to_string()),
                        toc_item.title.clone(),
                        is_active,
                        is_active || is_open,
                    );
                    current_toc.children = children;
                    if is_open {
                        found_here = true;
                    }
                    current_toc
                };

                toc_list.push(current_toc.clone());

                if current_page.is_none() {
                    found_here = fpm::sitemap::Sitemap::ids_matches(toc_item.id.as_str(), id);
                    if found_here {
                        if let Some(ref title) = toc_item.nav_title {
                            current_toc.title = Some(title.to_string());
                        }
                        *current_page = Some(current_toc);
                    }
                }
            }
            (found_here, toc_list)
        }

        fn get_url(id: &str) -> String {
            if id.eq("/") {
                return id.to_string();
            }
            let id = id.trim_start_matches('/');
            if id.ends_with('/') || id.ends_with("index.html") {
                return id.to_string();
            }
            format!("{}/", id)
        }
    }

    pub(crate) fn get_extra_data_by_id(
        &self,
        id: &str,
    ) -> Option<std::collections::BTreeMap<String, String>> {
        for section in self.sections.iter() {
            if fpm::sitemap::Sitemap::ids_matches(section.id.as_str(), id) {
                return Some(section.extra_data.to_owned());
            }
            if let Some(data) = get_extra_data_from_subsections(id, section.subsections.as_slice())
            {
                let mut all_data = section.extra_data.clone();
                all_data.extend(data);
                return Some(all_data);
            }
        }
        return None;

        fn get_extra_data_from_subsections(
            id: &str,
            subsections: &[Subsection],
        ) -> Option<std::collections::BTreeMap<String, String>> {
            for subsection in subsections {
                if subsection.visible
                    && fpm::sitemap::Sitemap::ids_matches(
                        subsection.id.as_ref().unwrap_or(&"".to_string()),
                        id,
                    )
                {
                    return Some(subsection.extra_data.to_owned());
                }
                if let Some(data) = get_extra_data_from_toc(id, subsection.toc.as_slice()) {
                    let mut all_data = subsection.extra_data.clone();
                    all_data.extend(data);
                    return Some(all_data);
                }
            }
            None
        }

        fn get_extra_data_from_toc(
            id: &str,
            toc: &[TocItem],
        ) -> Option<std::collections::BTreeMap<String, String>> {
            for toc_item in toc {
                if fpm::sitemap::Sitemap::ids_matches(toc_item.id.as_str(), id) {
                    return Some(toc_item.extra_data.to_owned());
                }
                if let Some(data) = get_extra_data_from_toc(id, toc_item.children.as_slice()) {
                    let mut all_data = toc_item.extra_data.clone();
                    all_data.extend(data);
                    return Some(all_data);
                }
            }
            None
        }
    }
}

#[derive(Debug)]
struct LevelTree {
    level: usize,
    item: TocItem,
}

impl LevelTree {
    fn new(level: usize, item: TocItem) -> Self {
        Self { level, item }
    }
}

fn construct_tree_util(mut elements: Vec<(SitemapElement, usize)>) -> Vec<Section> {
    let mut sections = vec![];
    elements.reverse();
    construct_tree_util_(elements, &mut sections);
    return sections;

    fn construct_tree_util_(
        mut elements: Vec<(SitemapElement, usize)>,
        sections: &mut Vec<Section>,
    ) {
        if elements.is_empty() {
            return;
        }
        let smallest_level = elements.last().unwrap().1;
        while let Some((SitemapElement::Section(section), _)) = elements.last() {
            sections.push(section.to_owned());
            elements.pop();
        }

        let last_section = if let Some(section) = sections.last_mut() {
            section
        } else {
            // todo: return an error
            return;
        };
        while let Some((SitemapElement::Subsection(subsection), _)) = elements.last() {
            last_section.subsections.push(subsection.to_owned());
            elements.pop();
        }

        let last_subsection = if let Some(subsection) = last_section.subsections.last_mut() {
            subsection
        } else {
            last_section.subsections.push(Subsection {
                visible: false,
                ..Default::default()
            });
            last_section.subsections.last_mut().unwrap()
        };

        let mut toc_items: Vec<(TocItem, usize)> = vec![];
        while let Some((SitemapElement::TocItem(toc), level)) = elements.last() {
            toc_items.push((toc.to_owned(), level.to_owned()));
            elements.pop();
        }
        toc_items.push((TocItem::default(), smallest_level));
        // println!("Elements: {:#?}", elements);
        let mut tree = construct_tree(toc_items, smallest_level);
        let _garbage = tree.pop();
        last_subsection
            .toc
            .extend(tree.into_iter().map(|x| x.item).collect::<Vec<TocItem>>());

        construct_tree_util_(elements, sections);
    }
}

fn get_top_level(stack: &[LevelTree]) -> usize {
    stack.last().map(|x| x.level).unwrap()
}

fn construct_tree(elements: Vec<(TocItem, usize)>, smallest_level: usize) -> Vec<LevelTree> {
    let mut stack_tree = vec![];
    for (toc_item, level) in elements.into_iter() {
        if level < smallest_level {
            panic!("Level should not be lesser than smallest level");
        }
        if !(stack_tree.is_empty() || get_top_level(&stack_tree) <= level) {
            let top = stack_tree.pop().unwrap();
            let mut top_level = top.level;
            let mut children = vec![top];
            while level < top_level {
                loop {
                    if stack_tree.is_empty() {
                        panic!("Tree should not be empty here")
                    }
                    let mut cur_element = stack_tree.pop().unwrap();
                    if stack_tree.is_empty() || cur_element.level < top_level {
                        // Means found children's parent, needs to append children to its parents
                        // and update top level accordingly
                        // parent level should equal to top_level - 1
                        assert_eq!(cur_element.level as i32, (top_level as i32) - 1);
                        cur_element
                            .item
                            .children
                            .append(&mut children.into_iter().rev().map(|x| x.item).collect());
                        top_level = cur_element.level;
                        children = vec![];
                        stack_tree.push(cur_element);
                        break;
                    } else if cur_element.level == top_level {
                        // if popped element is same as already popped element it is adjacent
                        // element, needs to push into children and find parent in stack
                        children.push(cur_element);
                    } else {
                        panic!(
                            "Stacked elements level should never be greater than top element level"
                        );
                    }
                }
            }
            assert!(level >= top_level);
        }
        let node = LevelTree::new(level, toc_item);

        stack_tree.push(node);
    }
    stack_tree
}
