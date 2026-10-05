# CBCoin, EXP và vòng đời cá — luật mới

Tài liệu này thay thế luật tiền/EXP/tăng trưởng trong các ghi chú bàn giao trước.

## Luật đã code

- CBCoin là tiền game mua cá. Kéo sò về thuyền tự động quy đổi: trắng 1, đỏ 10, tím 100 CBCoin. Bộ sưu tập vẫn ghi số lượng từng loại. Ngọc trai là phần thưởng sưu tầm riêng, không thay đổi giá trị sò.
- Cho ăn chỉ nhận EXP, không nhận tiền. MB dùng hệ thập phân: 1 MB = 1.000.000 byte. Công thức EXP/file: `(floor(bytes / 20.000.000) + 1) × 20`. Do dùng điều kiện `<`, file đúng 20 MB nhận 40 EXP, đúng 40 MB nhận 60 EXP. File rỗng cũng nằm bậc 20 EXP.
- Không còn hệ số tuổi file, loại file, giảm thưởng mỗi ngày hoặc trần EXP ngày. Các kiểm tra loại/vị trí/dung lượng an toàn vẫn áp dụng; tối đa 4 GiB/file như trước.
- File mới dùng SHA-256 toàn bộ nội dung + kích thước, đọc theo chunk trên máy. Copy/đổi tên vẫn là cùng nội dung. Lịch sử đã trả EXP không bị cắt bỏ; cả restore rồi cho ăn lại cũng không nhận thêm.
- Preview lọc nội dung trùng giữa các file được chọn và lịch sử đã thưởng. Backend kiểm tra lại và không chuyển file trùng. File đang no hoặc cá trưởng thành cũng bị chặn trước khi chuyển file; một lượt nhiều file dừng cấp thức ăn khi chạm mốc nghỉ, các file còn lại ở nguyên chỗ.
- Cá mua mới bắt đầu Lv.0, cá non. **10 EXP = 1 level** (từ save schema 5; trước đó 100 EXP). Lv.50 (500 EXP) là thành niên, Lv.100 (1.000 EXP) là trưởng thành và không tăng thêm. Một file nhỏ (20 EXP) đã bằng 2 level.
- Mỗi mốc 5 level (50 EXP), cá nghỉ 7.200 giây. EXP dư của file đã ăn được giữ trong `pending_exp`, không vứt đi. Hết nghỉ, phần dư được tiêu hóa đến mốc nghỉ tiếp theo; nếu lại đủ 5 level thì nghỉ tiếp 2 tiếng. Không tiếp tục nhận file lúc nghỉ. Phần EXP vượt tổng 1.000 không ghi nhận.
- Thời hạn nghỉ được lưu để sống qua restart; manager/tank kiểm tra lại khoảng 15 giây khi app mở. Đồng hồ hệ thống được dùng cho thời hạn; chưa có cơ chế chống người dùng chỉnh đồng hồ.
- Bể hiện bong bóng vui khi nghỉ, ví dụ “No căng vảy! Cho em ngủ tí”, “Bụng em thành bóng rồi!”. Chế độ họp không hiện bong bóng.
- Lv.100 có nút gọi thuyền bán cá, xác nhận và hoạt cảnh thuyền đến đón. Giá bán = giá mua đã lưu ×100 CBCoin; backend kiểm tra tuổi, tràn số, chống bán lặp và ghi tiền/xóa cá trong cùng save atomic.
- Cá starter cũng có thể bán khi trưởng thành; giá gốc là giá shop của loài đó. Bán cá không đụng tới file trong Bụng cá, các file vẫn khôi phục được.
- Giữ quà khởi đầu 40 CBCoin, giá shop hiện có, sức chứa bể và hạn mức đào 60 sò/ngày. Hạn mức tính số sò, không tính CBCoin.

## Sinh sản (bản dùng thử)

Hai cá **cùng loài**, cả hai trưởng thành (Lv.100), được ghép cặp bằng nút "Sinh sản" trên thẻ cá. Người chơi chọn số trứng.

- **Giá trị bán:** mỗi trứng trừ của *từng* cá bố mẹ đúng một lần giá mua gốc. Giá bán hiện tại = giá mua × (100 − số trứng đã đẻ). Số trứng cộng dồn theo từng cá (`eggs_used`) và tối đa 99, tức giá bán không bao giờ thấp hơn giá mua gốc. Khi một trong hai cá hết hạn mức, chỉ còn đẻ được số trứng còn lại của cá ít hạn mức hơn.
- **Tỷ lệ nở mỗi trứng:** giảm tuyến tính theo giá của loài, từ 20% (giá ≤ 20 CBCoin) xuống 5% (giá ≥ 300 CBCoin). Cấu hình trong `balance.json` mục `breeding`. Trứng không nở vẫn bị trừ giá trị.
- **Cá con:** nở ra là cá bột Lv.0, `origin = hatched`, có `parent_ids` của hai cá bố mẹ và `generation` = đời lớn nhất của bố mẹ + 1. Giá mua ghi nhận bằng giá catalog của loài, nên cá con nuôi lên Lv.100 bán được như cá mua mới.
- **Bể đầy:** trứng được xử lý lần lượt; khi bể đầy, các trứng chưa dùng được giữ lại, không bị trừ giá trị. Bể đầy ngay từ đầu thì không sinh sản được.
- Nở tức thì ngay khi bấm; chưa có thời gian ấp, trạng thái trứng trong bể, tính trạng hay biến thể màu. Save cũ không có `eggs_used` đọc ra 0, không cần đổi schema.

## Code quà tặng (Cài đặt → Nhập code)

Code không phân biệt hoa thường, bỏ khoảng trắng hai đầu; game chỉ lưu SHA-256 của code trong mã nguồn (`game.rs`). Hai code admin để test, dùng lại được nhiều lần:

- `TQ-ADMIN-1TY-CB-2610`: +1.000.000.000 CBCoin.
- `TQ-ADMIN-FULLEXP-2610`: mọi cá trong bể lên Lv.100 (trưởng thành, hết nghỉ).

Đây là code thử nghiệm: ai biết code đều dùng được, nên cần xóa hoặc đổi trước khi phát hành chính thức.

## Tương thích dữ liệu

Schema save mới là 5: EXP và EXP đang tiêu hóa của save 4 được chia 10 nên level giữ nguyên. Từ schema 4: wallet đổi sang `cbcoins` và đọc được trường `shells` cũ, giữ số dư 1:1. Không đổi dữ liệu thật trong lúc test.

Save 1–3 giữ ID cá, ledger, collection và số dư. Cá trưởng thành cũ chuyển Lv.100; cá thành niên cũ giữ giai đoạn tương ứng Lv.50 trở lên; cá non cũ quy đổi tiến độ từ ngưỡng cũ 20/60 EXP. Không tạo lại quà khởi đầu. Save cũ không lưu giá mua thực tế nên dùng giá catalog của loài tại lúc migration; cá mua mới lưu giá giao dịch.

Lịch sử cũ chỉ có hash phần đầu: tiếp tục dùng nó để chặn thưởng lặp với dữ liệu đã ghi. Vì không thể phục hồi toàn bộ nội dung từ hash cũ, hai file khác nhau cùng kích thước/phần đầu có thể bị từ chối nếu khớp lịch sử cũ. File được thưởng từ luật mới dùng hash toàn bộ, không thêm hash phần đầu vào ledger nữa.

Không mở save schema 5 bằng EXE cũ. Save phiên bản tương lai hoặc hỏng vẫn fail-closed/read-only, không ghi đè.

## Kiểm thử và cách chạy

- `npm run build`: PASS TypeScript + Vite.
- `cargo test --lib` tại `src-tauri`: 60/60 PASS trên Windows, gồm dung lượng biên, từng màu sò, EXP dư, nghỉ/restart, hai mốc tuổi, migration, bán lặp, save lỗi, tràn tiền, copy/đổi tên, nội dung cùng đầu nhưng khác đuôi, dừng batch khi no và khôi phục file.
- Sửa nhận diện đường dẫn Windows canonical `\\?\C:\...` để kiểm tra local drive bình thường; không mở cho UNC/network/device/ADS. Test bảo vệ các namespace và vị trí hệ thống PASS.
- Test filesystem chỉ dùng file tự tạo trong thư mục test tạm, không dùng file người dùng. Chưa kiểm tra trực quan bong bóng/hoạt cảnh bán bằng phiên bản cài đặt mới.

Source: `work/trashquarium-github`. Tại thư mục đó chạy `npm run tauri dev`. Đóng game cũ trước khi tự chạy cùng identifier. Có thể đặt `TRASHQUARIUM_DATA_DIR` sang thư mục QA riêng để không dùng save thật. Cần Node và Rust/MSVC theo cấu hình Tauri hiện có.

Chưa tạo installer release hoặc push GitHub trong đợt này. EXE cài trước đó không tự cập nhật theo source mới.
