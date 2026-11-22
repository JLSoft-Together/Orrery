# Spike layoutlib (v32 Phase 0)

Render layout XML chỉ dùng view framework ra PNG từ `main()` Kotlin thường (không JUnit), trên Windows 11.
**Kết quả: GO.** PNG đúng ở lần chạy đầu: `out/sample.png`, 1080x2340, Pixel 5, `Theme.Material.Light.NoActionBar`.

## Phiên bản đã ghim
| Thành phần | Phiên bản |
|---|---|
| `app.cash.paparazzi:paparazzi` | 2.0.0-alpha05.1 (Maven Central, 2026-09-28) |
| `com.android.tools.layoutlib:layoutlib` / `-runtime:win` / `-resources` | 16.2.3 (Android 16, API 36; đúng bản Paparazzi build cùng) |
| `layoutlib-api` / `sdk-common` / `common` (kéo theo) | 32.0.1 / 32.0.0 / 32.0.0 |
| Kotlin / Gradle | 2.3.0 / 8.14.3 (wrapper) |
| JDK | Android Studio JBR 21.0.10 (`java` trên PATH là 11, không đủ) |

## Chạy
```sh
export JAVA_HOME="/c/Program Files/Android/Android Studio/jbr"
./gradlew run                          # sample/sample.xml, 10 lần render
./gradlew run -Pxml=path/to/layout.xml
```
`run` giải nén runtime + resources vào `build/layoutlib/`, set `paparazzi.layoutlib.{runtime,resources}.root`.

## Cách làm
Dùng `PaparazziSdk` (public API) làm phần khởi tạo Bridge (`setup()` + `prepare()`, `Environment` dựng tay, không cần plugin Gradle).
Mỗi lần render: `KXmlParser` + `ILayoutPullParser` đọc XML từ chuỗi → bọc `BridgeXmlBlockParser` → `layoutInflater.inflate(parser, FrameLayout, false)` → `sdk.snapshot(view)` → `BufferedImage`.
Cookie của parser = số dòng, `BridgeContext.getViewKey(view)` trả lại được → có sẵn đường cho `viewTree` (class, dòng, bounds).

## Số đo (máy này, 2 lần chạy)
| | ms |
|---|---|
| `setup()+prepare()` (Bridge init, load framework res, font, ICU) | 3950–4430 |
| Render đầu tiên | 150–280 |
| JVM start → frame đầu | ~4300–4700 |
| Render ấm (inflate + measure/layout/draw), trung bình 10 lần | 39–43 (min 30, max 68) |
| Ghi PNG 1080x2340 bằng `ImageIO` | 200–290 |

## Dung lượng tải (Windows)
layoutlib.jar 57.5 MB + layoutlib-runtime-win 73 MB + layoutlib-resources 33.2 MB ≈ **164 MB** nén
(giải nén: runtime 134 MB + resources 102 MB). Thêm ~20–30 MB jar phụ (sdk-common, guava, byte-buddy, paparazzi 0.5 MB…).

## Gotcha
- `java` mặc định là 11 → phải trỏ JBR 21 (Paparazzi cần ≥ 11, layoutlib 16 build cho 17+).
- `getViewCookie()` dùng `lineNumber` của KXml = dòng có `>` đóng thẻ (vd LinearLayout báo 7, thẻ mở ở dòng 2). Phase 1 phải tự ghi vị trí `<` đầu thẻ.
- Sau `snapshot()` Paparazzi gỡ view khỏi root, nên bounds phải lấy trong/ngay sau render (left/top vẫn còn).
- Ghi PNG chiếm thời gian nhiều hơn render: Phase 1 nên dùng encoder nhanh (nén thấp) hoặc gửi ảnh nhỏ hơn.
- Log rất ồn (`ImageReader_JNI … maxImages`, `Aconfig … mmap`, style conflict) — vô hại, cần lọc khi chuyển thành `log` notification.
- Byte-buddy agent self-attach: cần `-XX:+EnableDynamicAgentLoading` (JDK 21 cảnh báo nếu thiếu).
- Paparazzi 2.0.0 vẫn alpha; gọi `PaparazziSdk` tiện nhưng nó mang theo logic test (logger `assertNoErrors` ở `teardown`). Daemon thật nên fork phần cần (Renderer, SessionParamsBuilder, resources) sau một adapter, đúng như rủi ro đã ghi trong plan.
- Chưa thử macOS (cần runtime `mac`/`mac-arm`, cùng code).

## Khuyến nghị
**GO cho Phase 1.** Render ấm ~40 ms ≪ ngưỡng 1 s; cold start ~4.5 s → warm-up khi mở project Android như plan.
Tải ~164 MB lần đầu là lớn nhưng hợp với quyết định "tải khi mở Design lần đầu".
