# QA — Đợt 01/20

## Đã kiểm tra

- 50 ID và 50 PNG được chọn; không thiếu đường dẫn trong manifest.
- Nhóm: 20 real / 15 mutation_fiction / 10 prehistoric_reconstruction / 5 mythology_inspired.
- Tất cả PNG đọc được và có pixel alpha bằng 0.
- Không có pixel alpha >= 128 tại cạnh ngoài của 50 file được chọn. Điều này kiểm tra chạm biên, không bảo đảm mọi khoảng trống đều rộng.
- Kích thước gốc có nhiều tỷ lệ: 1536×1024, 1374×1145, 1484×1060. Chưa ép đồng kích thước hay tạo atlas.
- Đã xem các kết quả tạo ảnh và sửa 2 mẫu: Pajama đột biến giữ đúng nhãn loài nền; Haikouichthys giảm hình miệng hút kiểu lamprey.
- Hai bản gốc giữ trong draft-variants/, không tính vào 50 mẫu và không nằm trong ZIP bàn giao.
- SHA-256 từng file và đo alpha/margin có trong qa-results.json.

## Hạn chế cần giữ khi bàn giao

- TQ1000-0007 có một góc alpha 1/255; TQ1000-0047 có hai góc alpha 1/255. Nhiễu gần trong suốt, chưa chỉnh bằng công cụ ảnh.
- Một số mẫu có margin sát, không đạt padding 12% yêu cầu trong prompt. Đặc biệt TQ1000-0032 top 8px / right 13px; TQ1000-0043 right 6px. Không bị cắt ở ngưỡng alpha 128 nhưng cần nới canvas trước rig/animation.
- Mắt và tỷ lệ được cách điệu; số tia vây, chi tiết răng, vảy, giáp và phục dựng tiền sử chưa được chuyên gia sinh học/cổ sinh xác nhận.
- Tên Việt là nhãn thiết kế, không phải danh pháp chuẩn. Tên khoa học chưa kiểm toán taxonomy trên toàn bộ từng loài.
- Văn Diêu là diễn giải mỹ thuật, nguồn văn bản gốc cần kiểm chứng thêm trước khi viết bài học Sơn Hải Kinh.
- Một số trình xem trong phiên hiển thị RGB ở pixel alpha=0, tạo cảm giác nền/glow. Đã kiểm tra trực tiếp alpha; giữ alpha khi nhập game, không ép PNG thành RGB/JPEG.
- Chưa kiểm thử gallery tương tác trong trình duyệt; đã kiểm tra cấu trúc và đường dẫn ảnh.
- Chưa có animation, atlas, game integration, kiểm thử hiệu năng 1.000 asset, upload GitHub hay PR cho bộ art.

## Chạy lại kiểm tra đọc-only trên Windows PowerShell 5.1

powershell.exe -NoProfile -ExecutionPolicy Bypass -File .\validate.ps1

Lựa chọn ExecutionPolicy này chỉ áp dụng tiến trình kiểm tra, không đổi chính sách hệ thống.
Script đọc file để đo alpha và hash, không sửa/xóa ảnh. Phiên này đã chạy thành công bằng Windows PowerShell 5.1.

