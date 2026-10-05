# Shell Hunt — cập nhật tính năng và QA

> Luật tiền/EXP và kết quả test bên dưới là mốc bàn giao trước. Luật hiện tại đã đổi sang CBCoin và save schema 4; xem [economy-growth.md](economy-growth.md).

> Ghi chú tích hợp: cửa sổ đào sò riêng đã được thay bằng cảnh đào sò vẽ thẳng trên bể desktop (`src/tank.ts`; điều khiển bằng dock nhỏ + Ctrl+Alt+Space). Animation móc đóng/mở, sò hiếm, ripple và thông báo bắt sò đã port sang đó; bộ sưu tập nằm ở tab đào sò trong manager. Nút bật/tắt âm thanh chưa port (chỉ có ở bản cửa sổ cũ).

## Đã triển khai

- Móc đóng khi thu dây/nhận thưởng, mở lại khi thả; hỗ trợ reduced motion và giữ trạng thái khi pause. Animation dùng PNG hiện có, không sửa ảnh gốc.
- Hiệu ứng nước, viền vàng cho sò hiếm, thông báo phần thưởng. Âm thanh tổng hợp có nút bật/tắt, mặc định tắt.
- Ba loại sò trong bộ sưu tập. Nhặt lần đầu mở thẻ tên khoa học, kiến thức, ngày tìm thấy, tổng số và số sò hiếm. Có cả trong manager và cửa sổ đào sò.
- Mỗi sò có xác suất hiếm 8%; sò hiếm có xác suất chứa ngọc trai 35% (2,8% tổng thể). Đây là cân bằng game, không phải xác suất sinh học. Giá trị được chọn một lần lúc sinh và lưu cùng sò.
- Mỗi lần bắt hợp lệ nhận 1 vỏ sò; giới hạn thưởng đào sò hiện có là 60/ngày. Ngọc trai hiện chỉ để thu thập, chưa dùng mua đồ.
- Thưởng, bộ sưu tập và trạng thái đã bắt được ghi cùng giao dịch lưu. Lặp lại yêu cầu không cấp thêm; lỗi lưu, tràn số hoặc hết hạn mức không cấp thưởng.
- Schema save 3: nâng cấp save 1/2; giữ ID/vị trí sò cũ, không quay lại phần thưởng hiếm. Bộ sưu tập và ngọc trai ban đầu rỗng/0. Không dùng EXE phiên bản cũ để tiếp tục save schema mới.
- Chừa vùng vẽ cho cột buồm và đưa trạng thái xuống dưới sau khi kiểm tra Computer Use. Không đổi tọa độ va chạm phía backend.

## Nội dung khoa học

Ảnh AI chỉ là minh họa, không dùng định danh ngoài tự nhiên. Các sinh vật này là thân mềm biển, không phải cá. Thẻ sử dụng kiến thức từ Marine Life Information Network:

- Pecten maximus: https://www.marlin.ac.uk/species/detail/1398
- Aequipecten opercularis: https://www.marlin.ac.uk/species/detail/1997/1000
- Mimachlamys varia: https://www.marlin.ac.uk/species/detail/2086

## Kiểm thử

- `npm run build`: PASS.
- `node scripts/test-hunt-motion.mjs`: PASS (đóng/mở, pause, reduced motion, dt âm, cân bằng save/restore canvas).
- `node scripts/test-hunt-art.mjs`: PASS (PNG, kích thước và anchor).
- `cargo test --lib` trong `src-tauri`: 36/52 PASS. Toàn bộ 22 test game/hunt PASS, gồm migration, chống thưởng lặp, reload, giới hạn ngày, tràn số và metadata sinh sò.
- 16 test còn FAIL thuộc App/Belly/FileGuard, liên quan đường dẫn canonical Windows bị nhận thành device_path. Các module đó không được sửa trong đợt tính năng này; chưa xác minh baseline riêng để khẳng định lỗi có từ trước. Không coi toàn bộ bộ test là đạt.
- Native Windows: manager, cửa sổ đào sò, sò hiếm và dialog bộ sưu tập hiển thị được. Browser QA: ba tư thế móc và ba thẻ đã mở hiển thị được.
- Chưa xác nhận bằng thao tác UI một lượt bắt sò hoàn chỉnh, chưa nghe kiểm tra âm thanh. Không tắt cơ chế tự pause khi mất focus để phục vụ test.
- Profile QA riêng có sò hiếm/ngọc trai được seed để quan sát, không đại diện xác suất thực tế. Không cho ăn/xóa file người dùng; không sửa profile của game release đang mở.

## Chạy source local

Working directory:

`C:\Users\LENOVO\Documents\Codex\2026-09-20\referenced-chatgpt-conversation-this-is-an-2\work\trashquarium-github`

Lệnh thông thường: `npm run tauri dev` (đóng phiên bản game cũ trước khi tự chạy).

Để dùng profile QA biệt lập đã tạo trên máy này, trong PowerShell tại working directory:

```powershell
$env:TRASHQUARIUM_DATA_DIR = Join-Path (Get-Location) '.runtime-check/hunt-v2'
npm run tauri dev -- --config .runtime-check/tauri-huntqa.json
```

Config QA dùng identifier `vn.craftbyte.trashquarium.huntqa`, không đổi identifier sản phẩm. `.runtime-check/` được gitignore, không phải dữ liệu bàn giao hay save sản phẩm. Preview không cấp thưởng: `npm run dev`, rồi mở `http://localhost:1420/scripts/hunt-preview.html`.

Chưa đóng gói installer mới, commit hay push GitHub trong đợt này.
