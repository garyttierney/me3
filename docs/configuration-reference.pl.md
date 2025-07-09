---
comments: false
hide:
  - nawigacja
---

# Dokumentacja konfiguracji Profilu Moda (.me3)

## Czym jest konfiguracja Profilu Moda?

Konfiguracja **Profilu Moda** to wersjonowany plik TOML, który informuje me3, które mody należy załadować, jak je załadować i jakie gry obsługują. Pełni funkcję manifestu konfiguracji modów, zawierając listę pakietów nadpisujących zasoby oraz natywnych bibliotek DLL, z opcjonalnie określonym porządkiem ładowania.

- **Sposób użycia:** me3 odczytuje ModProfile, aby wiedzieć, które mody załadować i w jakiej kolejności. Możesz uruchomić profil, klikając go dwukrotnie (Windows) lub za pomocą wiersza poleceń (`me3 launch --profile my-profile.me3`).
- **Wersjonowanie:** Pole `profileVersion` zapewnia kompatybilność starszych profili po wprowadzeniu zmian niezgodnych wstecz.
- **Elastyczność:** Profile mogą być przechowywane w dowolnym miejscu, odwoływać się do ścieżek względnych lub bezwzględnych i są kompatybilne z nowymi funkcjami me3

## Przykładowa konfiguracja

```toml
profileVersion = "v1"

[[packages]]
id = "my-cool-texture-pack"
path = 'mods/MyCoolTexturePack/'

[[packages]]
id = "my-cool-model-pack"
path = 'mods/MyCoolTexturePack/'

[[natives]]
path = 'mods/MyAwesomeMod.dll'
```

## Analiza przykładowej konfiguracji

- **profileVersion**: Jest to wersja me3, dla której ten profil został napisany. Pozwala to na poprawne działanie starszych profili po wprowadzeniu zmian niezgodnych wstecz w formacie profilu.
- **[[packages]]**: Każdy blok definiuje pakiet nadpisywania zasobów. `id` to unikalna nazwa pakietu, a `path` wskazuje folder zawierający pliki moda. Możesz dodać wiele pakietów, dodając więcej bloków `[[packages]]`, każdy z unikalnym `id`. Zwróć uwagę, że używamy tutaj pojedynczych cudzysłowów, aby nie było konieczności stosowania znaków ucieczki dla ukośników odwrotnych w ścieżkach Windows.
- **[[natives]]**: Każdy blok definiuje natywny mod DLL do załadowania. `path` wskazuje plik DLL. Możesz dodać wiele natywnych modów, dodając kolejne bloki `[[natives]]`.

## Opis referencyjny

Poniżej znajduje się renderowana wersja schematu profilu moda.

--8<-- "schemas/mod-profile.md"
