# TypeScript — testy Swaplock testnet, 2026-09-29

Wynik po poprawce: **84/84 sprawdzenia** — 21 sprawdzeń × dwa węzły ×
Node.js/Chromium. Szczegółowe czasy, bloki i wyniki znajdują się w
[raporcie JSON](TYPESCRIPT-TESTNET-2026-09-29.json).

Portal `https://portal.swaplock.chainpool.online/` wskazuje w opublikowanym
bundlu `app.3bbe39e8106c64c08c4f.js`:

- `wss://node01.swaplock.chainpool.online:8090`
- `wss://node02.swaplock.chainpool.online:8090`

Oba zwróciły chain ID zgodny z generowaną specyfikacją:
`3b4f346481a66146d732eb22a43b5956134e5bea6124b379033589146036cbbf`.
Testy Chromium odbyły się o 13:22 UTC, na bloku 1201337; końcowe testy Node
o 13:24 UTC, na blokach 1201371–1201372. Chromium załadował rzeczywisty portal
i uruchomił bundle testowy z jego originu, bez polyfilli Node.

Zakres: login/discovery API, chain/config/global properties, harmonogram opłat,
konta i full accounts, aktywa, salda, routing obiektów i zachowanie null dla
brakujących ID, blok/nagłówek, wycena transferu, Data Room/access state/members,
Content Card/lista kart pokoju. Pokój `1.23.0` i karta `1.26.0` istniały.
Odpowiedzi dekodowano wygenerowanymi kodekami, a parametry kodowano generowanymi
deskryptorami RPC; liczby przechodziły przez bezstratny parser JSON.

Pierwszy przebieg wykrył brak `extended_asset_object.id` w IR: ekstraktor
spłaszczał dziedziczone pola C++, ale pomijał ID dodawane do klasy bazowej.
Naprawiono ekstraktor, dodano regresję dla dwóch poziomów dziedziczenia
(typed i generic ID), zregenerowano specyfikację Swaplock oraz bindingi Rust/TS.
Po zmianie asercja obecności ID i brakującego aktywa przechodzi na obu węzłach.

Przy odtwarzaniu generacji wykryto też, że stary snapshot CI `50179db...`
nie zawierał `get_data_room_access_state`, mimo obecności tej metody w zapisanej
specyfikacji. Zaktualizowano snapshot i pin CI do czystego źródła
`7b471be9d1cc051e7a3563a639a9ee98979de9d8`. Archiwum obejmuje wyłącznie
wybrane katalogi kodu i licencję. Sprawdzono SHA-256 oraz identyczność
wygenerowanej specyfikacji z rozpakowanego archiwum. Regeneracja z przypiętego
BitShares Core pozostała bez zmian.

Regresja lokalna: 441 testów Rust, 11 testów Node, testy typów TS, kontrola
regeneracji oraz Clippy przeszły. Domyślny CI nadal działa offline względem
testnetu; testy live uruchamia się jawnie:

```sh
cd open-graphene-packages/typescript
pnpm test:testnet
pnpm test:testnet:browser
```

Nie wykonywano podpisów ani broadcastu. Nie odczytywano prywatnych kluczy
genesis. Testy korzystają z małego adaptera WebSocket przeznaczonego wyłącznie
do testów, więc nie weryfikują jeszcze produkcyjnego transportu SDK,
reconnect/subskrypcji ani pełnego zakresu operacji. Wycena transferu potwierdza
zgodność JSON i dekodowania opłat, nie poprawność podpisanej transakcji.
