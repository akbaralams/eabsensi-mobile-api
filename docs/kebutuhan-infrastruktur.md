# Kebutuhan infrastruktur: 2.000 vs 64.000 pegawai

Daftar kebutuhan (spesifikasi dan jumlah) untuk menyusun RAB. Harga sengaja tidak dicantumkan. Angka diturunkan dari uji beban 29 Oktober 2026 ([load-test.md](load-test.md)), bukan tebakan. Sebelum pengadaan final, ulangi uji beban di VM target.

## Asumsi

| Asumsi | Nilai |
|---|---|
| Hari kerja | 22 hari/bulan |
| Punch per pegawai per hari | 2 (masuk + pulang), masing-masing dengan foto |
| Upacara (check-in event dengan foto) | 4 kali/bulan |
| Ukuran foto | 150 KB (bisa turun ke ±50 KB bila aplikasi mengompres foto sebelum kirim) |
| Jendela puncak pagi upacara | semua pegawai dalam 5 menit |
| Login ulang saat puncak | 10% (token masih berlaku); dengan refresh token di `/api/v2`, mendekati 0% |
| Biaya login (bcrypt cost 12) | ±0,23 detik CPU per login |
| Kapasitas terukur 1 proses API | ±1.000 request/detik (200 pegawai/detik alur pagi penuh), p99 punch 6 ms, memori 64 MB |

## Beban puncak (pagi upacara)

| | 2.000 pegawai | 64.000 pegawai |
|---|---:|---:|
| Pegawai per detik | 7 | 213 |
| Request API per detik | ±35 | ±1.070 |
| Insert punch + check-in per detik | ±13 | ±430 |
| Login per detik (10% login ulang) | 0,7 | 21 |
| Inti CPU untuk login (utilisasi 70%) | < 1 | ±7 |
| Lalu lintas unggah foto | ±16 Mbps | ±510 Mbps (±170 Mbps bila foto 50 KB) |

## Pertumbuhan data

| | 2.000 pegawai | 64.000 pegawai |
|---|---:|---:|
| Foto per bulan (150 KB) | ±14 GB | ±460 GB |
| Foto per tahun (150 KB) | ±170 GB | ±5,5 TB |
| Foto per tahun (50 KB) | ±57 GB | ±1,8 TB |
| Baris punch per tahun | ±1,1 juta | ±34 juta |
| Ukuran database per tahun (data + indeks) | ±1 GB | ±25 GB |

Kebijakan retensi foto sangat menentukan biaya penyimpanan (mis. foto disimpan 1 tahun, lalu dihapus atau dipindah ke arsip murah). Perlu keputusan bersama bagian kepegawaian.

## Skenario A: 2.000 pegawai

Satu server aplikasi cukup; server kedua untuk database dan cadangan.

| Komponen | Jumlah | Spesifikasi | Fungsi |
|---|---:|---|---|
| Server aplikasi | 1 | 4 vCPU, 8 GB RAM, SSD 100 GB | nginx (TLS), Laravel (PHP-FPM) untuk web operator, API Rust (2 proses untuk restart bergiliran) |
| Server database | 1 | 4 vCPU, 8 GB RAM, SSD 200 GB | PostgreSQL 18 |
| Penyimpanan foto | 1 | 500 GB (cukup ±3 tahun pada 150 KB) | disk server aplikasi, atau bucket S3-compatible |
| Backup | 1 | ±1 TB, lokasi terpisah | dump database harian + salinan foto |
| Sertifikat TLS | 1 | domain absensi | |
| Internet | 1 | 50 Mbps simetris | cukup untuk puncak ±16 Mbps |

Tidak perlu Redis, load balancer khusus, atau server monitoring terpisah (log JSON + `/healthz` cukup). Titik lemah: server aplikasi dan database tunggal; bila butuh ketersediaan tinggi, tambahkan satu server aplikasi dan replika database seperti skenario B dalam skala kecil.

## Skenario B: 64.000 pegawai

Semua komponen minimal dua (tidak ada titik gagal tunggal). Instance API stateless, jadi ditambah/dikurangi tanpa pengguna logout.

| Komponen | Jumlah | Spesifikasi | Fungsi |
|---|---:|---|---|
| Load balancer | 2 | 2 vCPU, 4 GB RAM (nginx + keepalived, IP virtual) | TLS, `/api` ke API, sisanya ke web. Bisa diganti F5 yang sudah ada |
| Server API (Rust) | 3 | 8 vCPU, 8 GB RAM, SSD 40 GB | 2 untuk beban puncak + 1 cadangan (N+1). Tambah sampai ±10 hanya bila 100% pegawai harus login ulang dalam 5 menit (dihindari dengan refresh token) |
| Server web (Laravel) | 2 | 4 vCPU, 8 GB RAM, SSD 60 GB | web operator, laporan, antrean job |
| PostgreSQL | 2 | 16 vCPU, 64 GB RAM, NVMe 1 TB | primary + replika (failover); PgBouncer di depannya |
| Redis | 3 | 2 vCPU, 4 GB RAM | batas laju bersama antar-instance API, Sentinel untuk failover |
| Object storage S3-compatible | 1 klaster | kapasitas pakai ±6 TB/tahun (150 KB) atau ±2 TB (50 KB), replikasi/erasure coding | foto punch dan check-in, dibaca web lewat disk S3 Laravel. On-premise: mis. MinIO 4 node; atau layanan cloud |
| Monitoring | 1 | 4 vCPU, 8 GB RAM, SSD 500 GB | metrik (CPU, req/dtk, p95, koneksi DB), log terpusat, alarm |
| Backup | 1 | ±2× ukuran DB + sesuai retensi foto, lokasi terpisah | backup database (PITR/WAL) + salinan bucket |
| Internet | 1 | 1 Gbps (minimal 500 Mbps) | puncak unggah foto saat upacara |
| Sertifikat TLS | 1 | domain absensi | |

Bila semua 64.000 pegawai harus login ulang dalam 5 menit (tanpa refresh token, atau token kedaluwarsa serentak), kebutuhan CPU login naik menjadi ±70 inti: itu alasan refresh token di `/api/v2` dan runbook pre-login di [deployment.md](deployment.md), bukan menambah server.

## Perbandingan singkat

| | 2.000 | 64.000 |
|---|---|---|
| Jumlah server | 2 | 13 + klaster storage + backup |
| Total vCPU (tanpa storage) | 8 | ±80 |
| Total RAM (tanpa storage) | 16 GB | ±200 GB |
| Penyimpanan foto per tahun | ±170 GB | ±5,5 TB (±1,8 TB dengan kompresi) |
| Internet | 50 Mbps | 1 Gbps |
| Ketersediaan tinggi | opsional | wajib (semua komponen minimal 2) |

## Keputusan yang memengaruhi biaya

1. **Ukuran foto**: kompres di aplikasi ke ±50 KB memangkas storage dan bandwidth hingga dua pertiga (perubahan di aplikasi Flutter, tanpa ubah kontrak).
2. **Retensi foto**: berapa lama foto disimpan.
3. **Refresh token**: menghilangkan lonjakan login, menahan jumlah server API di 3.
4. **On-premise vs cloud** untuk object storage dan backup.
5. **F5 yang sudah ada** bisa menggantikan dua VM load balancer.
