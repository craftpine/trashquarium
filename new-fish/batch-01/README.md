# TrashQuarium — Bộ 1.000 thiết kế · Đợt 01/20

Đợt này: TQ1000-0001 đến TQ1000-0050. 50 thiết kế mới, không tính lại 70 mẫu biển/invertebrate trước đó.
Cơ cấu mỗi đợt dự kiến: 20 loài thật, 15 đột biến hư cấu, 10 phục dựng tiền sử, 5 cảm hứng thần thoại.
Toàn bộ chương trình: 20 đợt × 50 = 1.000 mẫu thiết kế, KHÔNG phải 1.000 loài khoa học.

## Dùng bộ ảnh

- Mở gallery.html sau khi giải nén để xem toàn bộ, tìm theo tên/mã và đổi nền sáng/tối.
- sprites/: các PNG gốc được tạo bằng công cụ tạo ảnh tích hợp (built-in image_gen), giữ nguyên alpha; không chuyển sang API/CLI.
- manifest.json: ID ổn định, tên Việt/taxon nếu phù hợp, nhóm, loài nền, đặc điểm, nguồn tham khảo và prompt đầy đủ.
- QA.md / qa-results.json: kết quả kiểm tra và hạn chế.
- Đây là ảnh tĩnh concept. Chưa có animation, skeleton, atlas, cơ chế di truyền hoặc tích hợp shop/Fishdex trong mã game.

## Ranh giới khoa học và giả tưởng

Tên Việt là nhãn UI, không phải tên phân loại chuẩn. Taxon cần dev/biên tập viên xác nhận trước khi xuất bản.
Loài thật: giữ đặc trưng nhận diện nhưng mắt, tỷ lệ và chi tiết được cách điệu, chưa được chuyên gia duyệt.
Tiền sử: phục dựng nghệ thuật; màu và nhiều phần mềm cơ thể không thể xác định chắc chắn từ hóa thạch.
Đột biến: thiết kế gameplay, không khẳng định thay đổi này có thể di truyền hay sống được ngoài tự nhiên.
Thần thoại: diễn giải sáng tạo có ghi nền văn hóa, không coi là sinh vật thật hoặc một bản hình dung duy nhất.
Không viết bài học rằng cá hiện đại lai với nhau sẽ sinh ra loài tuyệt chủng hoặc thần thoại.
Tiktaalik/Cheiracanthus không được mặc định phân loại là cá biển chỉ vì nằm trong bộ asset thủy sinh.

## Nguồn tham khảo

- [NHM — Prehistoric fish timeline](https://www.nhm.ac.uk/discover/prehistoric-fish-timeline.html): tham khảo hình thái phục dựng; không sao chép ảnh bảo tàng.
- [LACMA — Hippocamp](https://collections.lacma.org/object/62714).
- [Rubin Museum — Makara](https://rubinmuseum.org/projecthimalayanart/glossary/makara/).
- [Asian Civilisations Museum — Life in Edo labels](https://www.nhb.gov.sg/acm/-/media/acm/acm2017/documents/life-in-edo-russel-wong-in-kyoto/life-in-edo_label-translations_2nd-rotations_eng.pdf?la=en): nguồn cho cảm hứng Namazu.
- [British Museum — Object A_1948-0414-1](https://www.britishmuseum.org/collection/object/A_1948-0414-1): ghi chú về shachi.
- [Chinese Text Project — 文鰩魚](https://ctext.org/dictionary.pl?char=%E6%96%87%E9%B0%A9%E9%AD%9A&if=en&remap=gb): trang tham khảo Văn Diêu bị chặn truy cập đầy đủ trong phiên này; cần kiểm chứng văn bản trước nội dung giáo dục.
Các nguồn là tư liệu định hướng, không phải giấy chứng nhận độ chính xác từng pixel.

## Tiến độ và bàn giao

Đợt 02 đến 20 chưa tạo. Không có tác vụ tự chạy nền.
Giữ ID qua các lần sửa; dùng tên -v2/-v3 cho phiên bản mới và cập nhật file trong manifest.
Không gán chi phí shop, rarity hoặc xác suất sinh sản ngẫu nhiên trong bộ art này: đó là dữ liệu gameplay cần cân bằng riêng.

