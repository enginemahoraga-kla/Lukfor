//! Extract the real shell icon of an app entry (.lnk/.url/.exe) as a
//! base64 PNG data URL, so the frontend can show the actual logo.

/// Extract the icon for `path`. Returns a `data:image/png;base64,...` URL,
/// or None if the shell has no icon for it (frontend falls back to a glyph).
pub fn extract(path: &str) -> Option<String> {
    #[cfg(windows)]
    {
        // Index paths can carry mixed separators (PathBuf::join on a
        // forward-slash literal); the shell refuses to resolve .lnk icons
        // for such paths, so normalize first.
        let p = path.replace('/', "\\");
        // Store/UWP entries are "shell:AppsFolder\<AUMID>" — not a file, so
        // SHGetFileInfoW can't reach them; use the shell image factory.
        if p.to_lowercase().starts_with("shell:appsfolder\\") {
            return win::appsfolder_icon_data_url(&p);
        }
        win::icon_data_url(&p)
    }
    #[cfg(not(windows))]
    {
        let _ = path;
        None
    }
}

#[cfg(windows)]
mod win {
    use base64::Engine;
    use windows::core::PCWSTR;
    use windows::Win32::Graphics::Gdi::{
        DeleteObject, GetDC, GetDIBits, GetObjectW, ReleaseDC, BITMAP, BITMAPINFO,
        BITMAPINFOHEADER, DIB_RGB_COLORS,
    };
    use windows::Win32::Foundation::SIZE;
    use windows::Win32::Graphics::Gdi::HBITMAP;
    use windows::Win32::Storage::FileSystem::FILE_FLAGS_AND_ATTRIBUTES;
    use windows::Win32::System::Com::{CoInitializeEx, COINIT_APARTMENTTHREADED};
    use windows::Win32::UI::Shell::{
        IShellItemImageFactory, SHCreateItemFromParsingName, SHGetFileInfoW, SHFILEINFOW,
        SHGFI_ICON, SHGFI_LARGEICON, SIIGBF_BIGGERSIZEOK, SIIGBF_ICONONLY,
    };
    use windows::Win32::UI::WindowsAndMessaging::{DestroyIcon, GetIconInfo, HICON, ICONINFO};

    pub fn icon_data_url(path: &str) -> Option<String> {
        unsafe {
            // The shell needs COM to resolve .lnk icons; S_FALSE (already
            // initialized) is fine.
            let _ = CoInitializeEx(None, COINIT_APARTMENTTHREADED);
            let wide: Vec<u16> = path.encode_utf16().chain(std::iter::once(0)).collect();
            let mut sfi = SHFILEINFOW::default();
            let ret = SHGetFileInfoW(
                PCWSTR(wide.as_ptr()),
                FILE_FLAGS_AND_ATTRIBUTES(0),
                Some(&mut sfi),
                std::mem::size_of::<SHFILEINFOW>() as u32,
                SHGFI_ICON | SHGFI_LARGEICON,
            );
            if ret == 0 || sfi.hIcon.is_invalid() {
                return None;
            }
            let png = hicon_to_png(sfi.hIcon);
            let _ = DestroyIcon(sfi.hIcon);
            png.map(|bytes| {
                format!(
                    "data:image/png;base64,{}",
                    base64::engine::general_purpose::STANDARD.encode(bytes)
                )
            })
        }
    }

    /// Icon for a shell namespace item like "shell:AppsFolder\<AUMID>"
    /// (Store/UWP apps). SHGetFileInfoW can't reach these, so ask the shell
    /// image factory to render the app's logo into an HBITMAP.
    pub fn appsfolder_icon_data_url(path: &str) -> Option<String> {
        unsafe {
            let _ = CoInitializeEx(None, COINIT_APARTMENTTHREADED);
            let wide: Vec<u16> = path.encode_utf16().chain(std::iter::once(0)).collect();
            let factory: IShellItemImageFactory =
                SHCreateItemFromParsingName(PCWSTR(wide.as_ptr()), None).ok()?;
            let hbitmap = factory
                .GetImage(SIZE { cx: 64, cy: 64 }, SIIGBF_ICONONLY | SIIGBF_BIGGERSIZEOK)
                .ok()?;
            let png = hbitmap_to_png(hbitmap);
            let _ = DeleteObject(hbitmap.into());
            png.map(|bytes| {
                format!(
                    "data:image/png;base64,{}",
                    base64::engine::general_purpose::STANDARD.encode(bytes)
                )
            })
        }
    }

    /// Convert a 32-bit HBITMAP (as returned by IShellItemImageFactory, which
    /// uses premultiplied alpha) into PNG bytes with straight-alpha RGBA.
    unsafe fn hbitmap_to_png(hbm: HBITMAP) -> Option<Vec<u8>> {
        let mut bm = BITMAP::default();
        if GetObjectW(
            hbm.into(),
            std::mem::size_of::<BITMAP>() as i32,
            Some(&mut bm as *mut _ as *mut _),
        ) == 0
        {
            return None;
        }
        let (w, h) = (bm.bmWidth, bm.bmHeight);
        if w <= 0 || h <= 0 || w > 512 || h > 512 {
            return None;
        }

        let mut bmi = BITMAPINFO::default();
        bmi.bmiHeader = BITMAPINFOHEADER {
            biSize: std::mem::size_of::<BITMAPINFOHEADER>() as u32,
            biWidth: w,
            biHeight: -h, // top-down rows
            biPlanes: 1,
            biBitCount: 32,
            biCompression: 0, // BI_RGB
            ..Default::default()
        };
        let mut buf = vec![0u8; (w as usize) * (h as usize) * 4];
        let hdc = GetDC(None);
        let lines = GetDIBits(
            hdc,
            hbm,
            0,
            h as u32,
            Some(buf.as_mut_ptr() as *mut _),
            &mut bmi,
            DIB_RGB_COLORS,
        );
        let _ = ReleaseDC(None, hdc);
        if lines == 0 {
            return None;
        }

        // GetImage gives premultiplied BGRA. Un-premultiply to straight alpha
        // (so edges aren't darkened in the <img>) and swap to RGBA. If the
        // bitmap carries no alpha at all, treat it as fully opaque.
        let has_alpha = buf.chunks_exact(4).any(|px| px[3] != 0);
        for px in buf.chunks_exact_mut(4) {
            if has_alpha {
                let a = px[3];
                if a != 0 && a != 255 {
                    let a = a as u16;
                    px[0] = ((px[0] as u16 * 255) / a).min(255) as u8;
                    px[1] = ((px[1] as u16 * 255) / a).min(255) as u8;
                    px[2] = ((px[2] as u16 * 255) / a).min(255) as u8;
                }
            } else {
                px[3] = 255;
            }
            px.swap(0, 2); // BGRA -> RGBA
        }
        encode_png(w as u32, h as u32, &buf)
    }

    unsafe fn hicon_to_png(hicon: HICON) -> Option<Vec<u8>> {
        let mut ii = ICONINFO::default();
        GetIconInfo(hicon, &mut ii).ok()?;
        let pixels = read_color_bitmap(&ii);
        let _ = DeleteObject(ii.hbmColor.into());
        let _ = DeleteObject(ii.hbmMask.into());
        let (w, h, rgba) = pixels?;
        encode_png(w, h, &rgba)
    }

    unsafe fn read_color_bitmap(ii: &ICONINFO) -> Option<(u32, u32, Vec<u8>)> {
        let mut bm = BITMAP::default();
        if GetObjectW(
            ii.hbmColor.into(),
            std::mem::size_of::<BITMAP>() as i32,
            Some(&mut bm as *mut _ as *mut _),
        ) == 0
        {
            return None;
        }
        let (w, h) = (bm.bmWidth, bm.bmHeight);
        if w <= 0 || h <= 0 || w > 512 || h > 512 {
            return None;
        }

        let mut bmi = BITMAPINFO::default();
        bmi.bmiHeader = BITMAPINFOHEADER {
            biSize: std::mem::size_of::<BITMAPINFOHEADER>() as u32,
            biWidth: w,
            biHeight: -h, // top-down rows
            biPlanes: 1,
            biBitCount: 32,
            biCompression: 0, // BI_RGB
            ..Default::default()
        };
        let mut buf = vec![0u8; (w as usize) * (h as usize) * 4];
        let hdc = GetDC(None);
        let lines = GetDIBits(
            hdc,
            ii.hbmColor,
            0,
            h as u32,
            Some(buf.as_mut_ptr() as *mut _),
            &mut bmi,
            DIB_RGB_COLORS,
        );
        let _ = ReleaseDC(None, hdc);
        if lines == 0 {
            return None;
        }

        // GDI gives BGRA; legacy icons may carry no alpha at all — treat
        // those as fully opaque instead of fully transparent.
        let has_alpha = buf.chunks_exact(4).any(|px| px[3] != 0);
        for px in buf.chunks_exact_mut(4) {
            px.swap(0, 2);
            if !has_alpha {
                px[3] = 255;
            }
        }
        Some((w as u32, h as u32, buf))
    }

    fn encode_png(w: u32, h: u32, rgba: &[u8]) -> Option<Vec<u8>> {
        let mut out = Vec::new();
        let mut enc = png::Encoder::new(&mut out, w, h);
        enc.set_color(png::ColorType::Rgba);
        enc.set_depth(png::BitDepth::Eight);
        let mut writer = enc.write_header().ok()?;
        writer.write_image_data(rgba).ok()?;
        writer.finish().ok()?;
        Some(out)
    }
}

#[cfg(all(test, windows))]
mod tests {
    use base64::Engine;

    const PNG_MAGIC: [u8; 8] = [0x89, b'P', b'N', b'G', 0x0D, 0x0A, 0x1A, 0x0A];

    fn assert_is_png_data_url(url: &str) {
        let b64 = url
            .strip_prefix("data:image/png;base64,")
            .expect("not a png data url");
        let bytes = base64::engine::general_purpose::STANDARD
            .decode(b64)
            .expect("invalid base64");
        assert_eq!(&bytes[..8], &PNG_MAGIC);
    }

    #[test]
    fn extracts_icons_from_most_start_menu_shortcuts() {
        // Some shortcuts point at shell namespaces (e.g. "Administrative
        // Tools.lnk") and legitimately have no extractable icon; the contract
        // is that ordinary app shortcuts work, so require a clear majority.
        let appdata = std::env::var("APPDATA").unwrap();
        let root = std::path::Path::new(&appdata).join("Microsoft/Windows/Start Menu/Programs");
        let lnks: Vec<_> = walkdir::WalkDir::new(&root)
            .into_iter()
            .filter_map(|e| e.ok())
            .filter(|e| {
                e.path()
                    .extension()
                    .map(|x| x.eq_ignore_ascii_case("lnk"))
                    .unwrap_or(false)
            })
            .take(15)
            .collect();
        assert!(!lnks.is_empty(), "no .lnk in start menu");

        let mut ok = 0;
        for lnk in &lnks {
            match super::extract(lnk.path().to_str().unwrap()) {
                Some(url) => {
                    assert_is_png_data_url(&url);
                    ok += 1;
                }
                None => eprintln!("no icon: {}", lnk.path().display()),
            }
        }
        assert!(
            ok * 2 > lnks.len(),
            "only {ok}/{} shortcuts yielded icons",
            lnks.len()
        );
    }

    #[test]
    fn extracts_icon_from_a_uwp_app() {
        // Enumerate the real packaged apps the same way the indexer does, then
        // require that a clear majority yield a valid PNG icon.
        let out = std::process::Command::new("powershell")
            .args([
                "-NoProfile",
                "-NonInteractive",
                "-Command",
                "Get-StartApps | ConvertTo-Json -Compress",
            ])
            .output()
            .expect("run Get-StartApps");
        let text = String::from_utf8_lossy(&out.stdout);
        let aumids: Vec<String> = text
            .split("\"AppID\":\"")
            .skip(1)
            .filter_map(|s| s.split('"').next())
            .filter(|id| id.contains('!'))
            .map(|id| id.to_string())
            .take(8)
            .collect();
        assert!(!aumids.is_empty(), "no packaged apps found via Get-StartApps");

        let mut ok = 0;
        for aumid in &aumids {
            match super::extract(&format!("shell:AppsFolder\\{aumid}")) {
                Some(url) => {
                    assert_is_png_data_url(&url);
                    ok += 1;
                }
                None => eprintln!("no icon: {aumid}"),
            }
        }
        assert!(
            ok * 2 > aumids.len(),
            "only {ok}/{} packaged apps yielded icons",
            aumids.len()
        );
    }

    #[test]
    fn extracts_icon_from_an_exe() {
        let url = super::extract("C:\\Windows\\System32\\notepad.exe").expect("no icon");
        assert_is_png_data_url(&url);
    }

    #[test]
    fn missing_file_yields_none_or_generic_icon_without_panicking() {
        // SHGetFileInfoW may still return a generic icon for a nonexistent
        // path; the contract here is just "no panic, valid output if Some".
        if let Some(url) = super::extract("C:\\definitely\\not\\here.exe") {
            assert_is_png_data_url(&url);
        }
    }
}
