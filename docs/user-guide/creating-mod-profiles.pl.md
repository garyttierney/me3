# Tworzenie Profili Modów

**Profil modów** określa, które mody mają zostać załadowane przez me3 i w jaki sposób. Niniejszy przewodnik opisuje process pobierania modów, konfigurowania lokalnego katalogu modów oraz tworzenia profilu modów.

Skonfigurujemy następujące mody DLL: [Fast Launch](https://www.nexusmods.com/eldenringnightreign/mods/30), [Nightreign Alt Saves](https://www.nexusmods.com/eldenringnightreign/mods/4) oraz [Disable Chromatic Aberration](https://www.nexusmods.com/eldenringnightreign/mods/67).

Do podmiany zawartości użyjemy następujących modów: [Fun Is Allowed](https://www.nexusmods.com/eldenringnightreign/mods/49) oraz [Geralt of Rivia over Wylder](https://www.nexusmods.com/eldenringnightreign/mods/63).

## Krok 1: Przygotuj katalog modów

- Wybierz folder, w którym będą przechowywane pliki modów. Domyślnie me3 przechowuje je w `%LOCALAPPDATA%/garyttierney/me3/config/profiles` lub `$HOME/.config/me3/profiles`, ale plik `.me3` może znajdować się w dowolnym miejscu poza dyskiem sieciowym.
- Utwórz folder o nazwie `mod`, aby przechowywać pobrane pliki modów.

## Krok 2: Dodaj swoje mody

- Umieść pliki zasobów (np. `regulation.bin`, foldery `parts/`) w `mod`.
- Umieść pliki `.dll` w folderze `natives`.
- Dla łatwiejszego zarządzania możesz tworzyć podfoldery w katalogu`mod` i odwoływać się do nich za pomocą oddzielnych wpisów `[[packages]]` w swoim profilu. To znacznie ułatwia dodawanie/usuwanie/aktualizowanie pojedynczych modów.

!!! tip "Jak działają ścieżki"
    Wszystkie ścieżki podane w profilu modów (`path` w `[[packages]]` i `[[natives]]`) odnoszą się do lokalizacji samego pliku `.me3`.
    Możesz przechowywać pliki modów w dowolnym miejscu, pod warunkiem, że pliku `.me3` użyjesz poprawnej ścieżki.



!!! warning "Kompatybilność natywnych modów"
    Niektóre natywne mody mogą mieć własne ograniczenia lub wymagania dotyczące ich konfiguracji. Upewnij się, że zapoznałeś się z dokumentacją każdego moda.

## Krok 3: Utwórz swój profil modów

Utwórz nowy plik (np. `myprofile.me3`) w folderze `Mods` z następującą zawartością:

```toml
profileVersion = "v1"

[[supports]]
game = "nightreign"

[[packages]]
id = "nightmods"
path = "mod"

[[natives]]
path = "natives/DisableChromaticAberration. ll"

[[natives]]
path = "natives/SkipIntroLogos.dll"

[[natives]]
path = "natives/nightreign_alt_saves.dll"
```

Ten profil deklaruje pakiet zamiany zasobów o nazwie `nightmods` (używając wszystkich plików z folderu `mod`) i wymienia każdy mod `.dll` w folderze `natives`. Deklarujemy również, że nasz profil obsługuje NIGHTREIGN. Dzięki temu me3 wie, którą grę skonfigurować po dwukrotnym kliknięciu w celu uruchomienia.

## Krok 4: Uruchom profil

Po skonfigurowaniu profilu możesz go uruchomić. Użytkownicy systemu Windows mogą dwukrotnie kliknąć plik `.me3`, aby uruchomić grę z modami, natomiast użytkownicy systemu Linux muszą uruchomić profil za pomocą CLI:

```shell
> $ me3 launch --auto-detect -p myprofile.me3
```

## Krok 5: Zagraj w zmodyfikowaną grę

![image](https://github.com/user-attachments/assets/9da0bf73-695d-4f0b-af83-2c88e6328fd3)
