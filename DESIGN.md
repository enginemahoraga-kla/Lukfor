# Lukfor: arah desain

Arah ini datang dari pemilik produk (jawaban 2026-09-26). Agent hanya merapikan formatnya.

## Arah dari pemilik

- **Karakter:** gaya ala game Valorant.
- **Warna aksen:** hijau zamrud / teal, ditambah sedikit oranye (2026-09-26).
  Pembagian tugasnya: teal menandai posisimu (sorotan, fokus, caret), oranye
  menandai milikmu (bintang favorit, label "Favorites") plus satu garis tipis
  di sudut potong panel. Oranye tidak pernah diletakkan di atas blok teal
  (kontrasnya 1,05:1).
  Satu pengecualian yang disengaja (2026-10-06): baris shutdown/restart/sleep
  yang sedang menunggu Enter kedua berubah jadi blok oranye penuh. Di situ
  oranye berarti "tombol berikutnya tidak bisa dibatalkan", jadi harus tampak
  beda dari sorotan teal biasa. Teks di atasnya 7,85:1 (gelap) dan 5,21:1 (terang).
- **Tema:** ikut setelan terang/gelap Windows.

## Batas

- Yang diambil dari Valorant adalah bahasa visualnya (sudut terpotong diagonal, huruf kondensed, blok aksen solid), bukan asetnya: tanpa logo, ikon, font, atau merek Riot Games.

## Design Read

Reading this as: launcher desktop Windows yang dibuka puluhan kali sehari, untuk pengguna yang ingin langsung mengetik dan pergi, dalam bahasa visual HUD game taktis (Valorant), dial **ENERGY 2 / RHYTHM 1 / MOTION 1**.

- **ENERGY 2:** karakter Valorant ada di bentuk dan satu blok aksen, bukan di banyak elemen. Launcher yang dibuka sepanjang hari tidak boleh berteriak.
- **RHYTHM 1:** daftar hasil memang seragam; itu keputusan, karena mata harus bisa memindai baris demi baris tanpa kejutan.
- **MOTION 2** (naik dari 1 atas permintaan pemilik, 2026-09-27): saat kolom
  kosong, kilatan tipis menyusuri garis teal di bawah kolom ketik tiap 4,2 detik
  dan placeholder bergilir menampilkan perintah. Anti-slop menandai loop tanpa
  pemicu (R-19); pemilik memilih mempertahankannya. Tujuannya: menandai panel
  siap diketik dan mengajarkan perintah. Batasnya: berhenti saat mengetik,
  saat panel tersembunyi, dan saat Windows meminta gerak dikurangi. Hanya
  `transform`/`opacity`. Navigasi keyboard tetap instan.
