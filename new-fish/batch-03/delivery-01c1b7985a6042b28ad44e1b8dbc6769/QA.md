# QA — Batch 03

50 concept tĩnh được chọn, kiểm tra bằng validate.ps1 chỉ đọc PNG, không chỉnh ảnh. Chi tiết kích thước, alpha, padding và SHA256 nằm trong qa-results.json. Ảnh nguyên bản tạo bằng built-in image_gen; hai bản sửa được chọn là 0102-v2 và 0139-v2. ZIP chỉ chứa ảnh được chọn, không đếm bản nháp thành mẫu mới.

Kết quả kỹ thuật: 50/50 file tồn tại, 50 SHA256 khác nhau, 50/50 có pixel hoàn toàn trong suốt; không có pixel alpha >=128 chạm biên ngoài. Padding nhỏ hơn 16 px ở 0119, 0129 và 0139-v2: nên tăng khoảng trống khi làm atlas, chưa coi là cắt mất bộ phận. Không thay đổi/resample ảnh trong bước kiểm tra.

Đã xem các ảnh khi công cụ trả kết quả. Đây là kiểm tra concept bằng mắt, không phải duyệt chuyên gia khoa học hoặc kiểm thử game. Gallery HTML chưa kiểm thử tương tác trong trình duyệt. Chưa có animation, rig, atlas, collider hoặc integration.

## Cần rà tiếp trước khi sản xuất

- 0101 và các cá sụn: artist kiểm tra chính xác số khe mang, số/vị trí vây; AI có thể thêm/bớt chi tiết.
- 0102: đã sửa mảng trắng từ đỉnh vây sang đầu tự do phía sau chân vây, vẫn cần chuyên gia duyệt.
- 0107: vị trí vây lưng còn quá gần giữa thân so với mục tiêu; cần sửa trước khi dùng trong Fishdex khoa học.
- 0136: silhouette còn giống cá mập hiện đại; cần rà lại cấu trúc vây/đuôi của Cladoselache.
- 0137–0138: rà cấu trúc vây hình đe và gai đực, không coi các mô mềm/màu là bằng chứng hóa thạch.
- 0139-v2: đã bỏ ba gai rời; cần làm rõ gai nâng đỡ ở mép trước hai vây lưng. Chưa đạt duyệt giải phẫu.
- 0140: rà vị trí/góc gai sau đầu và hình thái vây/đuôi Xenacanthus; là dạng nước ngọt.
- 0141: răng hiện còn dạng nút tròn, cần làm thành mặt nghiền dẹt với họa tiết phù hợp trước minh họa giáo dục.
- 0142–0145: rà hàm, răng, vảy và tỷ lệ theo tài liệu chuyên ngành.
- 0146–0150: biến thể mỹ thuật từ truyền thống đã có; không coi là hình tượng chuẩn hay năm truyền thuyết độc lập.

Toàn bộ nhóm loài thật/tiền sử còn chờ duyệt giải phẫu, danh pháp và nội dung kiến thức. Nguồn được ghi trong manifest/source-notes, không phải toàn bộ 50 mẫu đã xác minh riêng. Đột biến và tiến hóa ngược là hư cấu gameplay.
