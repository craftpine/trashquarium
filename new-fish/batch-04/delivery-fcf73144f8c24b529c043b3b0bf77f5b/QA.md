# TrashQuarium — QA đợt 04

Ngày bàn giao: 2026-10-04. Phạm vi: 50 concept tĩnh TQ1000-0151–0200, không phải 50 taxon mới hay hình giải phẫu đã duyệt.

## Kiểm tra file — đã đạt

- Có đủ 50 file được chọn theo manifest; 50 ID, 50 đường dẫn và 50 SHA256 khác nhau.
- PNG thực tế đều 1254 × 1254 px; cả 50 có alpha trong suốt thật. Không resample hoặc chỉnh pixel bằng mã.
- Không có pixel alpha >=128 chạm mép ngoài; khoảng trống nhỏ nhất theo ngưỡng alpha >=128 là 17 px. Không thấy đầu mút cơ thể bị cắt cụt trong lượt xem concept.
- Gallery có đúng 50 ảnh, đường dẫn khớp các file được chọn. Manifest, prompt ban đầu, prompt sửa và nguồn đi kèm.
- Dùng built-in image_gen qua skill imagegen, một lần gọi cho mỗi mẫu/biến thể. Không dùng CLI/API fallback. Bản sửa không tính thành mẫu mới.
- Số liệu, hash và khoảng trống từng ảnh nằm trong qa-results.json. ZIP được kiểm tra thêm bằng đối chiếu hash từng file sau giải nén khi đóng gói.

## Kiểm tra kỹ thuật — còn cần xử lý trước sản xuất

- Không mẫu nào đạt đủ 15% padding ở cả bốn cạnh như prompt mong muốn. Tất cả vẫn có khoảng trống theo ngưỡng alpha >=128, nhưng cần thêm canvas/padding khi tạo atlas, xoay hoặc animation.
- 0159, 0189, 0193, 0196 có một pixel góc alpha=1/255; ba góc còn lại alpha=0. Đây là nhiễu alpha rất thấp, không phải nền đen đặc, nhưng không ghi “mọi góc trong suốt tuyệt đối”.
- Cần kiểm viền bán trong suốt trên nền sáng/tối và trong renderer thật; kết quả alpha không bảo đảm không có color-fringing khi lọc texture.
- Chưa có rig, frame animation, atlas, collider, scale giữa loài, LOD hay kiểm thử hiệu năng/tích hợp game.

## Các bản sửa được chọn

0154-v2 giảm gai bên sao giòn; 0161-v2 chuyển mắt ốc sứ khỏi chóp xúc tu; 0162-v2 giữ bốn lỗ mở bào ngư; 0170-v2 làm rõ chi đập/pleopod; 0186-v3 và 0187-v3 sửa giáp bọ ba thùy; 0191-v2 sửa chelicerae/telson; 0195-v2 thêm nhánh lớn ở crown. Bản đầu và bản trung gian giữ nguyên ngoài gói; ZIP chỉ lấy một ảnh cuối cho mỗi ID.

Những cải thiện thị giác không đồng nghĩa tất cả mục tiêu giải phẫu đã đạt.

## Các vấn đề khoa học/hình thể còn mở

| Mã | Cần artist/chuyên gia rà tiếp |
|---|---|
| 0157 | Tâm petaloid của Dendraster còn gần giữa; chưa thể hiện rõ đặc điểm lệch tâm. |
| 0158 | Các ụ mặt lưng được cách điệu dày. |
| 0163 | Prompt ban đầu dùng “unequal ears”; không dùng chi tiết đó làm kiến thức Pecten maximus. Hai tai và phối cảnh cần rà theo nguồn. |
| 0164 | Phân biệt lỗ hút với lỗ thoát, không dùng hai ống tròn tương tự làm sơ đồ chuẩn. |
| 0169 | Chỉ thấy rõ hai đôi chân đi; đôi thứ ba có thể bị che. |
| 0170 | Bản sửa tốt hơn nhưng không xác nhận đủ số đôi chi qua phối cảnh. |
| 0186-v3 | Vẫn nhìn được 7 hàng ngực rời thay mục tiêu 8; cần chỉnh, không anatomy/count pass. |
| 0187-v3 | Có thể đếm khoảng 8 dải nhưng ranh đầu/occipital/thorax chưa rõ; chưa chứng nhận count 8. |
| 0189–0190 | Chưa xác nhận đủ 12 đốt bụng từ phối cảnh; chelicerae cần loài/reference cụ thể. |
| 0191-v2 | Vẫn thấy 10 tấm bụng thay mục tiêu 12; cần chỉnh, không anatomy/count pass. |
| 0195-v2 | Có nhánh chữ Y lớn, nhưng pattern năm tia/hai cấp phân nhánh chưa rõ; cup lớn và tuft cuối cuống cách điệu. |

Lượt xem và liên kết nguồn cụ thể nằm trong visual-review-real.md, visual-review-prehistoric.md, visual-review-mythology.md và research-notes.md. Các ảnh đột biến kế thừa ý tưởng loài nền nhưng có giải phẫu hư cấu; nguồn loài nền không chứng minh đột biến.

## Nhãn nội dung cần giữ

20 loài thật là thiết kế theo taxon, còn chờ duyệt giải phẫu. 15 đột biến và tiến hóa ngược là hư cấu gameplay, không phép lai tự nhiên. 10 tiền sử có màu/mô mềm suy đoán. 5 văn hóa là sáng tạo lấy cảm hứng, không thẩm định iconography/canon.

Không gọi sao biển, ốc, cua, huệ biển là cá trong thẻ kiến thức. Không suy mọi mẫu trong đợt là loài đáy biển nước mặn. Akupara không được trình bày như đã xác minh đội thế giới; sò Aphrodite là mô-típ vỏ sò chứ không loài thần thoại có tên riêng. Không dùng hình Heikegani để dạy anatomy chuẩn của Heikeopsis japonica.

## Kết luận

Hoàn thành 50 **concept tĩnh có chú thích giới hạn**, đủ để bàn giao chọn hướng mỹ thuật và lên kế hoạch chỉnh/rig. Chưa production-ready, chưa science/cultural approved, chưa tích hợp source/game/GitHub. Không sửa hoặc xóa dữ liệu cá nhân; các gói đợt cũ giữ nguyên.

