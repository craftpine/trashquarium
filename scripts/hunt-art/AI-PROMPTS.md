# Prompt tạo ảnh AI cho trò đào sò (thuyền / móc / vỏ sò)

Thay các file PNG trong `public/art/hunt/` bằng ảnh AI của bạn (giữ đúng tên + kích thước, nền trong suốt):

| File | Kích thước | Ghi chú |
|---|---|---|
| `boat.png` | 720x540 | thuyền nhìn ngang, hướng sang phải; lỗ thả dây ở đáy thân thuyền, ngay chính giữa (xấp xỉ 50% ngang, 83% dọc) |
| `claw.png` | 256x352 | móc ba ngạnh, thẳng đứng, đầu nối dây ở trên cùng; tâm cặp ở khoảng 71% chiều cao từ trên xuống |
| `shell_0.png`, `shell_1.png`, `shell_2.png` | 512x512 | vỏ sò, nhìn từ phía trước, nằm giữa khung, chừa lề ~10% |

Tạo xong: dùng công cụ xóa nền (nếu AI không xuất nền trong suốt) rồi resize đúng kích thước. Kiểm tra bằng `npm run tauri dev`.

## Style chung (dán vào cuối mọi prompt)

> hand-painted digital illustration, soft painterly shading, warm rim light from above, gentle texture, same style as a lush planted freshwater aquarium game, clean silhouette, isolated on a fully transparent background, no text, no watermark, no shadow on the ground

## Thuyền

> A small cute wooden fishing sailboat seen from the side, bow pointing right, cream canvas sail with a small red pennant on the mast, teal and cream striped hull with two round brass portholes, warm orange wood planks, a small round hole in the exact center of the bottom of the hull where a rope comes out, floating gently, 4:3 canvas. [style chung]

## Móc

> A vertical brass grabber claw with three curved prongs opening downward, a small ring on top for attaching a rope, polished warm gold metal with subtle highlights, slightly open, 8:11 canvas. [style chung]

## Vỏ sò (3 biến thể — chạy 3 lần, đổi màu)

> A single scallop seashell seen from the front, fan-shaped with radial ridges, hinge at the bottom, [MÀU] , glossy pearl highlights, centered with margin, square canvas. [style chung]

`[MÀU]` lần lượt: `cream white with soft beige ridges` / `coral pink with rose ridges` / `pearl lilac with pale violet ridges`.

## Mẹo

- Nếu muốn ảnh khác nhau giữa các lần: thêm `seed` cố định hoặc dùng ảnh hiện tại làm ảnh tham chiếu (image reference).
- Ảnh mới có tỉ lệ khác thì chỉnh hằng `BOAT_HATCH` / `CLAW_GRAB` trong `src/tank.ts`.
