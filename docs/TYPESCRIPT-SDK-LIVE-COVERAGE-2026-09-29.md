# Live SDK Swaplock — 29 września 2026

Wynik: **40 z 47 metod RPC potwierdzonych funkcjonalnie**, **7 zablokowanych przez politykę dostępu węzłów**, 0 pozostałych błędów. Zakres to aktualny SDK TypeScript, a nie pełna funkcjonalność Rust ani wszystkie operacje protokołu.

39 metod odczytu sprawdzono przez produkcyjny transport SDK w Node.js i Chromium (origin portalu), na node01 i node02: **156 pozytywnych wyników**. Dodatkowo 12 pozytywnych scenariuszy: limit opłaty, brak odbiorcy, nieistniejący pokój/karta/blok. Każda z 7 metod crypto otrzymała odmowę dostępu w każdym z 4 przebiegów; nie zaliczamy tych odmów jako testów funkcjonalnych kryptografii.

## Rzeczywista transakcja z TypeScript

- TX: `93d5cb63619b1e36d34c87e6a93dd30cc7d6d62a`
- [Blok 1202081](https://portal.swaplock.chainpool.online/block/1202081), 2026-09-29T14:00:12 UTC; indeks 0.
- `swaplock` → `registrar`: 0,01 BTS; opłata 2 BTS.
- Serializacja FC, digest, podpis i ID wykonane lokalnie przez TypeScript. Bajty podpisanej transakcji identyczne z C++, autoryzacja zweryfikowana przez węzeł.
- `prepareTransfer → PreparedTransfer.sign → SignedTransfer.toJSON → broadcast → waitForInclusion` wykonane; włączenie potwierdzone na obu węzłach, osiągnięty blok nieodwracalny.
- Zmiana salda nadawcy: −2,01 BTS; odbiorcy: +0,01 BTS.
- Broadcast wykonano raz w Node.js. Nie wykonywano dodatkowego broadcastu w przeglądarce.

## Dane testowe i skutki

Utworzono aktywo `OGTSMUMQHCL1` (`1.3.100`), wydano 1 jednostkę, wykonano małą wymianę między własnymi zleceniami oraz wystawiono zlecenie do testów głębokości rynku. Historia wykonań zawierała 2 pozycje, historia rynku 1 świecę, zlecenia grupowane 1 grupę.

Pokój z tematem aktywa: `1.23.267`. Osobny pokój z publicznym, testowym envelope: `1.23.268`, karta `1.26.126`, grant dla kontrolowanego konta registrar: `1.28.0`. Wszystkie trzy metody odczytu grantów zwróciły rzeczywisty grant. Dane nie zawierają poufnej treści.

Fixture setup używał natywnego serializatora C++ udostępnianego przez RPC, a następnie lokalnego podpisu. **Nie jest to dowód obsługi tych operacji przez serializer FC TypeScript**. Natywny zakres FC TypeScript nadal obejmuje transfer.

Po odczytach anulowano zlecenie `1.7.2`: TX `b37ed7b531dbf6f8b4683f73e175870fb2067a21`, blok 1202108; potwierdzono brak obiektu. Aktywo, pokoje, karta i grant pozostają jako oznaczone dane testowe. Zadeklarowane opłaty w potwierdzonych transakcjach przygotowania/porządkowania: 503.50009 BTS testnetowych, plus 2 BTS za transfer SDK (nie jest to wyliczenie opłat netto po mechanizmach zwrotu opłat zleceń).

Pierwsza próba utworzenia karty została odrzucona przez walidację z powodu pustego `storage_data`; poprawiona próba z `{}` została przyjęta. Dziennik zachowuje obie próby i nie ponawia nieznanych wyników broadcastu.

## Dlaczego crypto_api jest niedostępne

Sprawdzono wskazane przez użytkownika stacki:
`platforms/chainpool-platform/stacks/chainpool-platform-swaplock-network/chainpool-platform-swaplock-network-rpc-node01/main.go`
oraz odpowiednik node02. Oba używają obrazu `ghcr.io/swaplock/swaplock-core:access-cas-7b471be9` i `NewSeedNodeWithSharedPVC`. Nie konfigurują pliku dostępu API.

Wspólny komponent `mirrorboards-utils/mirrorboards-pulumi/blockchain/actaboards/seednodewithsharedPVC.go` nie przekazuje `--api-access`. Domyślna lista w `swaplock-core/libraries/app/application.cpp:313–322` zawiera database, broadcast, history, orders i custom_operations, ale **nie crypto_api**. Oba publiczne endpointy potwierdzają to błędem `is_allowed: Access denied`.

Nie zmieniono ani nie wdrożono infrastruktury. Dokończenie siedmiu testów wymaga endpointu z przyznanym dostępem do crypto_api; klucze genesis służą do podpisywania, a nie do nadawania uprawnień RPC.

## Poprawka wykryta przez live test

Historia rzeczywistego zlecenia ujawniła błędne rozpoznanie lokalnego aliasu C++ `extensions_type = extension<options_type>`. Ekstraktor mylił go z globalną tablicą przyszłych rozszerzeń. Poprawiono ekstraktor, zregenerowano wspólne specyfikacje i bindingi Rust/TypeScript dla Swaplock i BitShares, dostosowano Rust API. Dodano regresję izolacji aliasu i testy JSON opcji zleceń. Po poprawce historia i bloki ze zleceniami dekodują się poprawnie.

Walidacja: 442 testy Rust, 20 testów Node, typecheck, Clippy bez ostrzeżeń, rustfmt, kontrola regeneracji obu łańcuchów oraz test kryptografii/FC w Chromium — pozytywne.

## Macierz metod

| Metoda | Wynik |
|---|---|
| `crypto.blind` | BLOKADA — crypto_api: Access denied |
| `crypto.blind_sum` | BLOKADA — crypto_api: Access denied |
| `crypto.verify_sum` | BLOKADA — crypto_api: Access denied |
| `crypto.verify_range` | BLOKADA — crypto_api: Access denied |
| `crypto.range_proof_sign` | BLOKADA — crypto_api: Access denied |
| `crypto.verify_range_proof_rewind` | BLOKADA — crypto_api: Access denied |
| `crypto.range_get_info` | BLOKADA — crypto_api: Access denied |
| `database.get_objects` | PASS — 4/4 (2 węzły × Node/Chromium) |
| `database.get_block_header` | PASS — 4/4 (2 węzły × Node/Chromium) |
| `database.get_block` | PASS — 4/4 (2 węzły × Node/Chromium) |
| `database.get_account_balances` | PASS — 4/4 (2 węzły × Node/Chromium) |
| `database.get_accounts` | PASS — 4/4 (2 węzły × Node/Chromium) |
| `database.get_full_accounts` | PASS — 4/4 (2 węzły × Node/Chromium) |
| `database.get_chain_properties` | PASS — 4/4 (2 węzły × Node/Chromium) |
| `database.get_dynamic_global_properties` | PASS — 4/4 (2 węzły × Node/Chromium) |
| `database.get_global_properties` | PASS — 4/4 (2 węzły × Node/Chromium) |
| `database.get_limit_orders` | PASS — 4/4 (2 węzły × Node/Chromium) |
| `database.get_chain_id` | PASS — 4/4 (2 węzły × Node/Chromium) |
| `database.get_config` | PASS — 4/4 (2 węzły × Node/Chromium) |
| `database.get_key_references` | PASS — 4/4 (2 węzły × Node/Chromium) |
| `database.get_assets` | PASS — 4/4 (2 węzły × Node/Chromium) |
| `database.lookup_asset_symbols` | PASS — 4/4 (2 węzły × Node/Chromium) |
| `database.list_assets` | PASS — 4/4 (2 węzły × Node/Chromium) |
| `database.lookup_accounts` | PASS — 4/4 (2 węzły × Node/Chromium) |
| `database.get_ticker` | PASS — 4/4 (2 węzły × Node/Chromium) |
| `database.get_required_fees` | PASS — 4/4 (2 węzły × Node/Chromium) |
| `database.get_data_room_by_id` | PASS — 4/4 (2 węzły × Node/Chromium) |
| `database.get_data_room_access_state` | PASS — 4/4 (2 węzły × Node/Chromium) |
| `database.get_data_rooms_by_owner` | PASS — 4/4 (2 węzły × Node/Chromium) |
| `database.get_data_rooms_by_subject` | PASS — 4/4 (2 węzły × Node/Chromium) |
| `database.get_data_room_members` | PASS — 4/4 (2 węzły × Node/Chromium) |
| `database.get_data_room_member` | PASS — 4/4 (2 węzły × Node/Chromium) |
| `database.get_data_rooms_by_member` | PASS — 4/4 (2 węzły × Node/Chromium) |
| `database.get_data_room_key_epochs` | PASS — 4/4 (2 węzły × Node/Chromium) |
| `database.get_data_room_key_epoch` | PASS — 4/4 (2 węzły × Node/Chromium) |
| `database.get_content_card_by_id` | PASS — 4/4 (2 węzły × Node/Chromium) |
| `database.get_content_cards_by_room` | PASS — 4/4 (2 węzły × Node/Chromium) |
| `database.get_content_cards_by_author` | PASS — 4/4 (2 węzły × Node/Chromium) |
| `database.get_content_card_grants_by_card` | PASS — 4/4 (2 węzły × Node/Chromium) |
| `database.get_content_card_grant` | PASS — 4/4 (2 węzły × Node/Chromium) |
| `database.get_content_card_grants_by_grantee` | PASS — 4/4 (2 węzły × Node/Chromium) |
| `history.get_account_history` | PASS — 4/4 (2 węzły × Node/Chromium) |
| `history.get_fill_order_history` | PASS — 4/4 (2 węzły × Node/Chromium) |
| `history.get_market_history` | PASS — 4/4 (2 węzły × Node/Chromium) |
| `network_broadcast.broadcast_transaction` | PASS — podpis i broadcast TypeScript |
| `orders.get_tracked_groups` | PASS — 4/4 (2 węzły × Node/Chromium) |
| `orders.get_grouped_limit_orders` | PASS — 4/4 (2 węzły × Node/Chromium) |

## Powtórzenie

Instrukcje i jawne polecenia modyfikujące testnet znajdują się w [README TypeScript](../open-graphene-packages/typescript/README.md#full-live-rpc-inventory). Po anulowaniu zlecenia ponowny odczyt rynku może być pusty; dla pełnego dodatniego scenariusza rynku należy utworzyć nowy zestaw fixture z nową ścieżką pliku.

[Pełny raport JSON](TYPESCRIPT-SDK-LIVE-COVERAGE-2026-09-29.json) zawiera wyniki każdego przebiegu, publiczną transakcję, dziennik fixture i ograniczenia. Nie zawiera kluczy prywatnych.
