---
title: me3 wydany
date: 2025-05-25
categories:
  - Informacje o nowych wydaniach
authors:
  - gtierney
---

# me3 wydany

Wersja 0.2.0 me3 została wydana z podstawową funkcjonalnością oczekiwaną od programu ładującego modyfikacje.
W tym wpisie znajdziesz szczegółowe informacje dotyczące instalacji, konfiguracji oraz użytkowania nowej wersji.

!!! tip
    Szukasz instalatora? Zobacz [przewodnik użytkownika](../../user-guide/installation.md).

<!-- more -->

## Wprowadzenie

me3 to nowa iteracja projektów [ModEngine](https://github.com/soulsmods/ModEngine2), które istniały wcześniej.
Obsługuje wszystkie funkcje, jakich można oczekiwać od podstawowego programu ładującego modyfikacje (ładowanie nadpisanych plików i rozszerzeń DLL, generowanie zrzutów awaryjnych i logów), ale jest zbudowany na kilku nowych zasadach projektowych:

- Lepsze doświadczenie użytkownika dla osób korzystających z modów
- Stabilna integracja dla twórców modów
- Łatwość utrzymania dla deweloperów me3

### Lepszy doświadczenie użytkownika

W starszych wersjach ModEngine process konfiguracji modów był niedostatecznie opisany, łatwo prowadził do błędów i wywoływał niejasności dotyczące umieszczania plików w odpowiednich katalogach.
Celem me3 jest wyeliminowanie licznych błędów znanych z poprzednich wersji oraz sprawienie, by korzystanie z modyfikacji było mniej frustrujące.

#### Prostsze uruchamianie

me3 oferuje zarówno integrację z powłoką systemu Windows, jak i wieloplatformowy interfejs wiersza poleceń.
Zamiast używać skryptów wywołujących `modengine_launcher`, użytkownicy mogą dwukrotnie kliknąć plik `.me3`, aby go uruchomić, o ile profil obsługuje daną grę.

Użytkownicy, którzy nie korzystają z systemu Windows (lub preferują CLI), mogą uruchomić profil za pomocą interfejsu wiersza poleceń `me3`:

```shell
> $ me3 launch --profile modded-elden-ring --game er
```

!!! tip
    Możesz użyć opcji `--auto-detect` zamiast `--game` do automatycznego wykrywania gry do uruchomienia, jeśli profil obsługuje tylko jedną grę.

#### Organizacja modyfikacji

Dzięki me3 Profil Moda może być umieszczony w dowolnym miejscu i może odwoływać się do ścieżek względnych do własnego pliku konfiguracyjnego lub podawać ścieżki absolutne do lokalizacji w innym miejscu systemu plików.
Ujednoliciliśmy lokalizację Profili Modów (chociaż nadal mogą być umieszczone w dowolnym miejscu!), aby ułatwić znajdowanie i uruchamianie dostępnych profili.

```shell
> $ me3 profile list
eldenring-default.me3
nightreign-default.me3
```

Oznacza to również, że możemy tworzyć profile i przechowywać je w nowej, ujednoliconej lokalizacji:

```shell
> $ me3 profile create -g er my-new-profile
> $ me3 profile show my-new-profile
● Mod Profile
    Path: /home/gtierney/.config/me3/profiles/my-new-profile.me3
    Name: my-new-profile
● Supports
    ELDEN RING: Supported
● Natives
● Packages
```

### Stabilna integracja

me3 ma stabilne API dla twórców modów, którzy chcą udostępniać elementary integracji wraz ze swoimi modami i generować konfiguracje w trakcie działania.

#### Wersjonowany schemat Profilu Moda

Nasze dotychczasowe podejście do plików konfiguracyjnych modów nie uwzględniało ewolucji schematu, co stawia nas w sytuacji, w której nie możemy wprowadzać usprawnień do formatu, bez ryzyka naruszenia kompatybilności z istniejącymi użytkownikami.
Profile Modów są teraz wersjonowane i będą kompatybilne z przyszłymi wersjami me3.
Za każdym razem, gdy w schemacie konfiguracji pojawi się zmiana niekompatybilna wstecz, number `profileVersion` zostanie zwiększony, a profile z wcześniejszych wersji nadal będą działać.

#### Integracja z launcherem

`me3-launcher.exe` jest teraz odpowiedzialny za dołączanie biblioteki DLL hosta moda do gry i może być używany samodzielnie jako część niestandardowego launchera do uruchamiania wstępnie zweryfikowanych profili przy odpowiedniej konfiguracji zmiennych środowiskowych.

```shell
ME3_GAME_EXE=path/to/game.exe ME3_HOST_DLL=path/to/me3-mod-host.dll ME3_HOST_CONFIG_PATH=path/to/attach/config/file me3-launcher.exe
```

Zmienna środowiskowa `ME3_HOST_CONFIG_PATH` wskazuje na plik TOML zawierający listy wstępnie posortowanych natywnych plików i pakietów w tym samym formacie, jakiego oczekuje format Profilu Moda`.me3`.

### Łatwiejsze utrzymanie

Największą zmianą w porównaniu do poprzednich wersji ModEngine z perspektywy dewelopera jest to, że budowanie, testowanie i uruchamianie jest teraz możliwe za pomocą pojedynczego polecenia.
Deweloperzy mogą korzystać z tych samych narzędzi, co użytkownicy końcowi do uruchamiania gry, a projekt można zbudować przy użyciu odpowiedniego zestawu narzędzi oraz pojedynczego polecenia `cargo build`.

## Instalacja i użytkowanie

me3 zawiera instalatory zarówno dla systemów Windows, jak i Linux, które można znaleźć na [stronie z wydaniami](https://github.com/garyttierney/me3/releases/latest/).
Po uruchomieniu kreatora instalacji, sprawdź [przewodnik użytkownika](../../user-guide/creating-mod-profiles.md), aby uzyskać informacje na temat tworzenia profilu moda.

## Co dalej?

W planach są kolejne funkcje mające na celu zniesienie niektórych ograniczeń modyfikowania gier FROMSOFTWARE.
Kolejne zadania, którymi chciałbym się zająć, to:

- Testy integracyjne dla deweloperów me3
- Obsługa modów z konfliktującymi nadpisaniami BND, ale niekonfliktującymi wpisami BND
- Rozwiązanie do hostingu, dystrybucji i wyszukiwania Profili Modów

## Słowo końcowe

me3 nie zostałby wydany, gdyby nie wszyscy, którzy przez lata wnosili swój wkład w postaci kodu, dokumentacji, pomysłów i wielu innych rzeczy do różnych projektów ModEngine.
Wszystkim zaangażowanym – dziękuję, bez określonej kolejności:

- [Jari Vetoniemi](https://github.com/Cloudef) - za pracę nad wsparciem ModEngine2 dla Protona
- [William Tremblay](https://github.com/tremwil) - za opinie i spostrzeżenia dotyczące systemu hooków me3
- [Vincent Swarte](https://github.com/vswarte) - główny współtwórca i główny deweloper odpowiedzialny za wsparcie dla ELDEN RING
- [Dasaav](https://github.com/dasaav-dsv) - za wkład w rozwój ModEngine 2, poprawki błędów, opinie oraz bieżące prace rozwojowe
- [ividyon](https://github.com/ividyon) - za wkład w rozwój ModEngine2 i opinie na temat UX/dokumentacji
- [katalash](https://github.com/katalash) - oryginalny twórca ModEngine i koncepcji hooków VFS
- [horkrux](https://github.com/horkrux) - za poprawki interfejsu debugowania do Dark Souls 3 oraz wkład w rozwój ModEngine2
- Gote - za opinie na temat doświadczeń użytkowników końcowych i dokumentacji
- Oraz wszystkim pozostałym, którzy wspierali nas podczas pracy nad projektem.
