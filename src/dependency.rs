use crate::utils::HasElements;

#[derive(Debug, Clone)]
pub struct Dependency {
    pub package: fpm::Package,
    pub version: Option<String>,
    pub notes: Option<String>,
    pub alias: Option<String>,
    pub implements: Vec<String>,
}

impl Dependency {
    pub fn unaliased_name(&self, name: &str) -> Option<String> {
        if name.starts_with(self.package.name.as_str()) {
            Some(name.to_string())
        } else {
            match &self.alias {
                Some(i) => {
                    if name.starts_with(i.as_str()) {
                        self.unaliased_name(
                            name.replacen(i.as_str(), self.package.name.as_str(), 1)
                                .as_str(),
                        )
                    } else {
                        None
                    }
                }
                None => None,
            }
        }
    }
}

pub async fn ensure(base_dir: &camino::Utf8PathBuf, package: &mut fpm::Package) -> fpm::Result<()> {
    if package.translations.has_elements() && package.translation_of.is_some() {
        return Err(fpm::Error::UsageError {
            message: "Package cannot be both original and translation package. \
            suggestion: Remove either `translation-of` or `translation` from FPM.ftd"
                .to_string(),
        });
    }

    let mut downloaded_package = vec![package.name.clone()];

    if let Some(translation_of) = package.translation_of.as_mut() {
        if package.language.is_none() {
            return Err(fpm::Error::UsageError {
                message: "Translation package needs to declare the language".to_string(),
            });
        }
        translation_of
            .process(base_dir, &mut downloaded_package, true, true)
            .await?;
    }

    for dep in package.dependencies.iter_mut() {
        dep.package
            .process(base_dir, &mut downloaded_package, false, true)
            .await?;
    }

    for translation in package.translations.iter_mut() {
        if package.language.is_none() {
            return Err(fpm::Error::UsageError {
                message: "Package needs to declare the language".to_string(),
            });
        }
        translation
            .process(base_dir, &mut downloaded_package, false, false)
            .await?;
    }

    Ok(())
}

#[derive(serde::Deserialize, Debug, Clone)]
pub(crate) struct DependencyTemp {
    pub name: String,
    pub version: Option<String>,
    pub notes: Option<String>,
    pub implements: Vec<String>,
}

impl DependencyTemp {
    pub(crate) fn into_dependency(self) -> fpm::Result<fpm::Dependency> {
        let (package_name, alias) = match self.name.as_str().split_once(" as ") {
            Some((package, alias)) => (package, Some(alias.to_string())),
            _ => (self.name.as_str(), None),
        };
        Ok(fpm::Dependency {
            package: fpm::Package::new(package_name),
            version: self.version,
            notes: self.notes,
            alias,
            implements: self.implements,
        })
    }
}

impl fpm::Package {
    /// `process()` checks the package exists in `.packages` or `FPM_HOME` folder (`FPM_HOME` not
    /// yet implemented), and if not downloads and unpacks the method.
    ///
    /// This is done in following way:
    /// Download the FPM.ftd file first for the package to download.
    /// From FPM.ftd file, there's zip parameter present which contains the url to download zip.
    /// Then, unzip it and place the content into .package folder
    ///
    /// It then calls `process_fpm()` which checks the dependencies of the downloaded packages and
    /// then again call `process()` if dependent package is not downloaded or available
    pub async fn process(
        &mut self,
        base_dir: &camino::Utf8PathBuf,
        downloaded_package: &mut Vec<String>,
        download_translations: bool,
        download_dependencies: bool,
    ) -> fpm::Result<()> {
        use std::io::Write;
        // TODO: in future we will check if we have a new version in the package's repo.
        //       for now assume if package exists we have the latest package and if you
        //       want to update a package, delete the corresponding folder and latest
        //       version will get downloaded.

        // TODO: Fix this. Removing this because if a package has been downloaded as both an intermediate dependency
        // and as a direct dependency, then the code results in non evaluation of the dependend package
        // if downloaded_package.contains(&self.name) {
        //     return Ok(());
        // }

        let root = base_dir.join(".packages").join(self.name.as_str());

        // Just download FPM.ftd of the dependent package and continue
        if !download_translations && !download_dependencies {
            let (path, name) = if let Some((path, name)) = self.name.rsplit_once('/') {
                (base_dir.join(".packages").join(path), name)
            } else {
                (base_dir.join(".packages"), self.name.as_str())
            };
            let file_extract_path = path.join(format!("{}.ftd", name));
            if !file_extract_path.exists() {
                std::fs::create_dir_all(&path)?;
                let fpm_string = get_fpm(self.name.as_str()).await?;
                let mut f = std::fs::File::create(&file_extract_path)?;
                f.write_all(fpm_string.as_bytes())?;
            }
            return fpm::Package::process_fpm(
                &root,
                base_dir,
                downloaded_package,
                self,
                download_translations,
                download_dependencies,
                &file_extract_path,
            )
            .await;
        }

        // Download everything of dependent package
        if !root.exists() {
            // Download the FPM.ftd file first for the package to download.
            let fpm_string = get_fpm(self.name.as_str()).await?;

            // Read FPM.ftd and get download zip url from `zip` argument
            let download_url = {
                let lib = fpm::FPMLibrary::default();
                let ftd_document = match fpm::doc::parse_ftd("FPM", fpm_string.as_str(), &lib) {
                    Ok(v) => v,
                    Err(e) => {
                        return Err(fpm::Error::PackageError {
                            message: format!("failed to parse FPM.ftd: {:?}", &e),
                        });
                    }
                };

                ftd_document
                    .get::<fpm::config::PackageTemp>("fpm#package")?
                    .into_package()
                    .zip
                    .ok_or(fpm::Error::UsageError {
                        message: format!(
                            "Unable to download dependency. zip is not provided for {}",
                            self.name
                        ),
                    })?
            };

            let path = std::env::temp_dir().join(format!("{}.zip", self.name.replace("/", "__")));

            let start = std::time::Instant::now();
            print!("Downloading {} ... ", self.name.as_str());
            std::io::stdout().flush()?;
            // Download the zip folder
            {
                let mut response =
                    if download_url[1..].contains("://") || download_url.starts_with("//") {
                        reqwest::get(download_url.as_str())?
                    } else if let Ok(response) =
                        reqwest::get(format!("https://{}", download_url).as_str())
                    {
                        response
                    } else {
                        reqwest::get(format!("http://{}", download_url).as_str())?
                    };
                let mut file = std::fs::File::create(&path)?;
                // TODO: instead of reading the whole thing in memory use tokio::io::copy() somehow?
                let mut buf: Vec<u8> = vec![];
                response.copy_to(&mut buf)?;
                file.write_all(&buf)?;
                // file.write_all(response.text().await?.as_bytes())?;
            }

            let file = std::fs::File::open(&path)?;
            // TODO: switch to async_zip crate
            let mut archive = zip::ZipArchive::new(file)?;
            for i in 0..archive.len() {
                let mut c_file = archive.by_index(i).unwrap();
                let out_path = match c_file.enclosed_name() {
                    Some(path) => path.to_owned(),
                    None => continue,
                };
                let out_path_without_folder = out_path.to_str().unwrap().split_once("/").unwrap().1;
                let file_extract_path = base_dir
                    .join(".packages")
                    .join(self.name.as_str())
                    .join(out_path_without_folder);
                if (&*c_file.name()).ends_with('/') {
                    std::fs::create_dir_all(&file_extract_path)?;
                } else {
                    if let Some(p) = file_extract_path.parent() {
                        if !p.exists() {
                            std::fs::create_dir_all(p)?;
                        }
                    }
                    // Note: we will be able to use tokio::io::copy() with async_zip
                    let mut outfile = std::fs::File::create(file_extract_path)?;
                    std::io::copy(&mut c_file, &mut outfile)?;
                }
            }
            fpm::utils::print_end(format!("Downloaded {}", self.name.as_str()).as_str(), start);
        }
        let fpm_ftd_path = if root.join("FPM.ftd").exists() {
            root.join("FPM.ftd")
        } else {
            let doc = std::fs::read_to_string(&root.join("FPM.manifest.ftd"));
            let lib = fpm::FPMLibrary::default();
            match fpm::doc::parse_ftd("FPM.manifest", doc?.as_str(), &lib) {
                Ok(fpm_manifest_processed) => {
                    let k: String = fpm_manifest_processed.get("FPM.manifest#package-root")?;
                    let new_package_root = k
                        .as_str()
                        .split('/')
                        .fold(root.clone(), |accumulator, part| accumulator.join(part));
                    if new_package_root.join("FPM.ftd").exists() {
                        new_package_root.join("FPM.ftd")
                    } else {
                        return Err(fpm::Error::PackageError {
                            message: format!(
                                "Can't find FPM.ftd file for the dependency package {}",
                                self.name.as_str()
                            ),
                        });
                    }
                }
                Err(e) => {
                    return Err(fpm::Error::PackageError {
                        message: format!("failed to parse FPM.manifest.ftd: {:?}", &e),
                    });
                }
            }
        };
        return fpm::Package::process_fpm(
            &root,
            base_dir,
            downloaded_package,
            self,
            download_translations,
            download_dependencies,
            &fpm_ftd_path,
        )
        .await;

        async fn get_fpm(name: &str) -> fpm::Result<String> {
            let response_fpm = if let Ok(response_fpm) =
                reqwest::get(format!("https://{}/FPM.ftd", name).as_str())
            {
                if response_fpm.status().is_success() {
                    Some(response_fpm)
                } else {
                    None
                }
            } else if let Ok(response_fpm) =
                reqwest::get(format!("http://{}/FPM.ftd", name).as_str())
            {
                if response_fpm.status().is_success() {
                    Some(response_fpm)
                } else {
                    None
                }
            } else {
                None
            };
            match response_fpm {
                Some(mut response_fpm) => Ok(response_fpm.text()?),
                None => Err(fpm::Error::UsageError {
                    message: format!(
                        "Unable to find the FPM.ftd for the dependency package: {}",
                        name
                    ),
                }),
            }
        }
    }

    /// This function is called by `process()` or recursively called by itself.
    /// It checks the `FPM.ftd` file of dependent package and find out all the dependency packages.
    /// If dependent package is not available, it calls `process()` to download it inside `.packages` directory
    /// and if dependent package is available, it copies it to `.packages` directory
    /// At the end of both cases, `process_fpm()` is called again
    ///
    /// `process_fpm()`, together with `process()`, recursively make dependency packages available inside
    /// `.packages` directory
    ///
    #[async_recursion::async_recursion]
    async fn process_fpm(
        root: &camino::Utf8PathBuf,
        base_path: &camino::Utf8PathBuf,
        downloaded_package: &mut Vec<String>,
        mutpackage: &mut fpm::Package,
        download_translations: bool,
        download_dependencies: bool,
        fpm_path: &camino::Utf8PathBuf,
    ) -> fpm::Result<()> {
        let ftd_document = {
            let doc = std::fs::read_to_string(fpm_path)?;
            let lib = fpm::FPMLibrary::default();
            match fpm::doc::parse_ftd("FPM", doc.as_str(), &lib) {
                Ok(v) => v,
                Err(e) => {
                    return Err(fpm::Error::PackageError {
                        message: format!("failed to parse FPM.ftd 2: {:?}", &e),
                    });
                }
            }
        };
        let mut package = {
            let temp_package: fpm::config::PackageTemp = ftd_document.get("fpm#package")?;
            temp_package.into_package()
        };

        package.translation_status_summary = ftd_document.get("fpm#translation-status-summary")?;

        downloaded_package.push(mutpackage.name.to_string());

        package.fpm_path = Some(fpm_path.to_owned());
        package.dependencies = {
            let temp_deps: Vec<fpm::dependency::DependencyTemp> =
                ftd_document.get("fpm#dependency")?;
            temp_deps
                .into_iter()
                .map(|v| v.into_dependency())
                .collect::<Vec<fpm::Result<fpm::Dependency>>>()
                .into_iter()
                .collect::<fpm::Result<Vec<fpm::Dependency>>>()?
        };

        let auto_imports: Vec<String> = ftd_document.get("fpm#auto-import")?;
        let auto_import = auto_imports
            .iter()
            .map(|f| fpm::AutoImport::from_string(f.as_str()))
            .collect();
        package.auto_import = auto_import;
        package.fonts = ftd_document.get("fpm#font")?;
        package.sitemap = ftd_document.get("fpm#sitemap")?;

        if download_dependencies {
            for dep in package.dependencies.iter_mut() {
                let dep_path = root.join(".packages").join(dep.package.name.as_str());

                if downloaded_package.contains(&dep.package.name) {
                    continue;
                }
                if dep_path.exists() {
                    let dst = base_path.join(".packages").join(dep.package.name.as_str());
                    if !dst.exists() {
                        futures::executor::block_on(fpm::copy_dir_all(dep_path, dst.clone()))?;
                    }
                    fpm::Package::process_fpm(
                        &dst,
                        base_path,
                        downloaded_package,
                        &mut dep.package,
                        false,
                        true,
                        &dst.join("FPM.ftd"),
                    )
                    .await?;
                }
                dep.package
                    .process(base_path, downloaded_package, false, true)
                    .await?;
            }
        }

        if download_translations {
            if let Some(translation_of) = package.translation_of.as_ref() {
                return Err(fpm::Error::PackageError {
                    message: format!(
                        "Cannot translate a translation package. \
                    suggestion: Translate the original package instead. \
                    Looks like `{}` is an original package",
                        translation_of.name
                    ),
                });
            }
            for translation in package.translations.iter_mut() {
                let original_path = root.join(".packages").join(translation.name.as_str());
                if downloaded_package.contains(&translation.name) {
                    continue;
                }
                if original_path.exists() {
                    let dst = base_path.join(".packages").join(translation.name.as_str());
                    if !dst.exists() {
                        futures::executor::block_on(fpm::copy_dir_all(original_path, dst.clone()))?;
                    }
                    fpm::Package::process_fpm(
                        &dst,
                        base_path,
                        downloaded_package,
                        translation,
                        false,
                        false,
                        &dst.join("FPM.ftd"),
                    )
                    .await?;
                } else {
                    translation
                        .process(base_path, downloaded_package, false, false)
                        .await?;
                }
            }
        }
        *mutpackage = package;
        Ok(())
    }
}
