use std::sync::mpsc;
use std::time::Duration;

use tauri::Webview;

pub type Png = Result<Vec<u8>, String>;

const CAPTURE_WAIT: Duration = Duration::from_secs(20);
const HISTORY_WAIT: Duration = Duration::from_secs(3);

#[cfg(windows)]
mod platform {
    use std::sync::mpsc::Sender;

    use tauri::webview::PlatformWebview;
    use webview2_com::CapturePreviewCompletedHandler;
    use webview2_com::Microsoft::Web::WebView2::Win32::COREWEBVIEW2_CAPTURE_PREVIEW_IMAGE_FORMAT_PNG;
    use windows::core::BOOL;
    use windows::Win32::System::Com::{IStream, STREAM_SEEK_SET};
    use windows::Win32::UI::Shell::SHCreateMemStream;

    use super::Png;

    unsafe fn read_all(stream: &IStream) -> Png {
        stream.Seek(0, STREAM_SEEK_SET, None).map_err(|e| e.to_string())?;
        let mut out = Vec::new();
        let mut chunk = vec![0u8; 65536];
        loop {
            let mut read = 0u32;
            let hr = stream.Read(chunk.as_mut_ptr().cast(), chunk.len() as u32, Some(&mut read));
            if hr.is_err() || read == 0 {
                break;
            }
            out.extend_from_slice(&chunk[..read as usize]);
        }
        Ok(out)
    }

    unsafe fn start(p: &PlatformWebview, tx: Sender<Png>) -> Result<(), String> {
        let core = p.controller().CoreWebView2().map_err(|e| e.to_string())?;
        let stream: IStream = SHCreateMemStream(None).ok_or("stream")?;
        let target = stream.clone();
        let handler = CapturePreviewCompletedHandler::create(Box::new(move |res| {
            let out = match res {
                Ok(()) => unsafe { read_all(&target) },
                Err(e) => Err(e.to_string()),
            };
            let _ = tx.send(out);
            Ok(())
        }));
        core.CapturePreview(COREWEBVIEW2_CAPTURE_PREVIEW_IMAGE_FORMAT_PNG, &stream, &handler).map_err(|e| e.to_string())
    }

    pub fn capture(p: &PlatformWebview, tx: Sender<Png>) {
        let fail = tx.clone();
        if let Err(e) = unsafe { start(p, tx) } {
            let _ = fail.send(Err(e));
        }
    }

    pub fn history(p: &PlatformWebview) -> (bool, bool) {
        unsafe {
            let Ok(core) = p.controller().CoreWebView2() else { return (false, false) };
            let (mut back, mut forward) = (BOOL(0), BOOL(0));
            let _ = core.CanGoBack(&mut back);
            let _ = core.CanGoForward(&mut forward);
            (back.as_bool(), forward.as_bool())
        }
    }
}

#[cfg(target_os = "macos")]
mod platform {
    use std::sync::mpsc::Sender;

    use block2::RcBlock;
    use objc2_app_kit::{NSBitmapImageFileType, NSBitmapImageRep, NSImage};
    use objc2_foundation::{NSDictionary, NSError};
    use objc2_web_kit::WKWebView;
    use tauri::webview::PlatformWebview;

    use super::Png;

    fn png_of(image: &NSImage) -> Png {
        let tiff = image.TIFFRepresentation().ok_or("tiff")?;
        let rep = NSBitmapImageRep::imageRepWithData(&tiff).ok_or("bitmap")?;
        let data = unsafe { rep.representationUsingType_properties(NSBitmapImageFileType::PNG, &NSDictionary::new()) }.ok_or("png")?;
        Ok(data.to_vec())
    }

    fn view(p: &PlatformWebview) -> &WKWebView {
        unsafe { &*(p.inner() as *const WKWebView) }
    }

    pub fn capture(p: &PlatformWebview, tx: Sender<Png>) {
        let block = RcBlock::new(move |image: *mut NSImage, _error: *mut NSError| {
            let out = match unsafe { image.as_ref() } {
                Some(image) => png_of(image),
                None => Err("snapshot".into()),
            };
            let _ = tx.send(out);
        });
        unsafe { view(p).takeSnapshotWithConfiguration_completionHandler(None, &block) };
    }

    pub fn history(p: &PlatformWebview) -> (bool, bool) {
        let wk = view(p);
        unsafe { (wk.canGoBack(), wk.canGoForward()) }
    }

    fn js_error(e: &NSError) -> String {
        use objc2_foundation::NSString;
        let key = NSString::from_str("WKJavaScriptExceptionMessage");
        e.userInfo().objectForKey(&key).and_then(|v| v.downcast::<NSString>().ok()).map(|s| s.to_string()).unwrap_or_else(|| e.localizedDescription().to_string())
    }

    pub fn call_js(p: &PlatformWebview, body: &str, tx: Sender<Result<String, String>>) {
        use objc2::runtime::AnyObject;
        use objc2::MainThreadMarker;
        use objc2_foundation::NSString;
        use objc2_web_kit::WKContentWorld;
        let Some(mtm) = MainThreadMarker::new() else {
            let _ = tx.send(Err("not on the main thread".into()));
            return;
        };
        let world = unsafe { WKContentWorld::pageWorld(mtm) };
        let block = RcBlock::new(move |res: *mut AnyObject, err: *mut NSError| {
            let out = match unsafe { err.as_ref() } {
                Some(e) => Err(js_error(e)),
                None => match unsafe { res.as_ref() } {
                    Some(o) => o.downcast_ref::<NSString>().map(|s| s.to_string()).ok_or_else(|| "result is not a string".to_owned()),
                    None => Ok("null".to_owned()),
                },
            };
            let _ = tx.send(out);
        });
        unsafe { view(p).callAsyncJavaScript_arguments_inFrame_inContentWorld_completionHandler(&NSString::from_str(body), None, None, &world, Some(&block)) };
    }
}

pub fn capture(webview: &Webview) -> Png {
    let (tx, rx) = mpsc::channel();
    webview
        .with_webview(move |p| platform::capture(&p, tx))
        .map_err(|e| e.to_string())?;
    rx.recv_timeout(CAPTURE_WAIT).map_err(|e| e.to_string())?
}

pub fn history(webview: &Webview) -> (bool, bool) {
    let (tx, rx) = mpsc::channel();
    if webview
        .with_webview(move |p| {
            let _ = tx.send(platform::history(&p));
        })
        .is_err()
    {
        return (false, false);
    }
    rx.recv_timeout(HISTORY_WAIT).unwrap_or((false, false))
}

#[cfg(target_os = "macos")]
pub fn call_js(webview: &Webview, body: String, wait: Duration) -> Result<String, String> {
    let (tx, rx) = mpsc::channel();
    webview.with_webview(move |p| platform::call_js(&p, &body, tx)).map_err(|e| e.to_string())?;
    rx.recv_timeout(wait).map_err(|_| "eval timeout".to_owned())?
}

#[cfg(not(target_os = "macos"))]
pub fn call_js(_webview: &Webview, _body: String, _wait: Duration) -> Result<String, String> {
    Err("callAsyncJavaScript is macOS only".into())
}
