# Auto Fan Management Utility in Linux Systems for Monster Laptops
# Monster Laptoplar için Linux Sistemlerde Oto Fan Yönetimi

<sub> TR </sub>

Monster laptoplar gömülü fan hızı olarak 90°C üstü ve altı olarak sadece 2 değer bulundurduğundan daha optimal bir çözüm için yazılmıştır.  
Fan hızı - CPU sıcaklığı değerleri Monster'ın Windows için olan Control Manager programından referans alınmıştır.  
Gömülü kontrollera low-level IO syscalları ile eriştiğinden **bu programı kullanmanız durumunda riskin size ait olduğunu bilin**.


# Kullanım
```
sudo ./monster-fans-linux
```


# Derleme
- rustc ve Cargo'yu indirin
```
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
```

- Git'i clonelayıp derleyin
```
git clone https://github.com/scrubjay55/monster-fans-linux
cd monster-fans-linux
cargo build --release
```
