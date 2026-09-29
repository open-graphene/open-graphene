# Natywny Open Graphene TypeScript — plan implementacji

Status: development rozpoczęty po akceptacji i przełączeniu modelu przez użytkownika (2026-09-29). Plan przygotowany na podstawie lokalnego repozytorium, HEAD `4d15808c0e2011b7fc7393c57853ff95c036b17f`.

Uzgodniony z użytkownikiem cel: natywny TypeScript dla Node.js i przeglądarki, pełny zakres aktualnego SDK Rust realizowany etapami, najpierw Swaplock. Użytkownik autoryzował development poleceniem „Przełączyłem. Rozpocznij development”.

Pierwszy przyrost obejmuje fundament E0–E2: inventory i publiczne wektory,
poprawkę generic object ID w Rust, wspólny profil codegen, workspace TS,
prymitywy/JSON oraz generator typów, kodeków i deskryptorów RPC obu sieci.
Weryfikacja obejmuje Node, Chromium, testy generatora i regresję Rust.
E0 nie jest jeszcze kompletną macierzą niezależnie uruchomionych wektorów
C++/bitsharesjs; dalsze wektory będą dochodzić z FC, podpisami i memo.
Generowane serializery FC, kryptografia podpisu/memo, transport i wyższe API
(E3–E8) pozostają do implementacji. `support.json` jawnie pokazuje te granice.
Warianty są na razie w `types.ts`/`json.ts`; osobny plik FC powstanie w E3.

Testy live 2026-09-29: 21 sprawdzeń na obu węzłach testnetu w Node i Chromium
przeszło po poprawce dziedziczonego `extended_asset_object.id` w ekstraktorze.
Snapshot CI zaktualizowano do `7b471be9d1cc051e7a3563a639a9ee98979de9d8`,
który odtwarza obecną specyfikację. Dane rewizji w części analitycznej poniżej
opisują historyczny punkt startowy. [Raport](TYPESCRIPT-TESTNET-2026-09-29.md).

Kolejny przyrost E3–E5 (2026-09-29): generowane FC dla transferu i transakcji,
WIF/low-S secp256k1, podstawowy transport RPC oraz prepare/sign/broadcast/inclusion.
Natywny transfer TS wszedł do nieodwracalnego bloku 1201598;
[dowody i zakres](TYPESCRIPT-LIVE-TRANSFER-2026-09-29.md).
Nie oznacza to ukończenia całych E3–E5: inne operacje FC, memo encryption,
multisig i zaawansowane sesje nadal pozostają do implementacji.

`open-graphene-packages/typescript` oznacza katalog w tym repozytorium, analogiczny do `open-graphene-packages/rust`. Nazwy npm poniżej są propozycją organizacyjną; nie sprawdzono własności scope ani nie zaplanowano automatycznej publikacji.

## 1. Jak działa obecny ekosystem

```text
blockchains/<chain>/<chain>-core — C++ konkretnego łańcucha
  + konfiguracja graphene-chain-<chain>-spec/open-graphene.toml
        |
        v
open-graphene-spec-gen
  discover -> extract facts -> resolve RPC/types -> build graph -> validate
        |
        v
Protocol JSON + model IR w open-graphene-json-schema
        |
        +--> open-graphene-gen-bindings-rs -> Rust src/generated/*.rs
        |
        +--> [plan] open-graphene-gen-bindings-ts -> TS src/generated/*.ts
```

1. `source/discover.rs` wybiera nagłówki z app/chain/protocol/plugins oraz pliki `.cpp` zawierające refleksję. `chain_repo_env` pozwala wskazać robocze źródło; CI używa przypiętego źródła.
2. `extract/` analizuje deklaracje C++, `FC_API`, rodziny `FC_REFLECT`, enumy, aliasy object ID, `static_variant` i oznaczenia operacji wirtualnych. To aktualnie dedykowane skanery/lexer, nie pełna analiza semantyczna kompilatorem C++; nie należy obiecywać obsługi dowolnego C++.
3. Refleksja ustala kolejność i wybór pól, deklaracje dostarczają typów. Uwzględniane są pola bazowe, zagnieżdżone typy i odziedziczone ID obiektów. Numery operacji pochodzą z pozycji/tagów `operation`, wartości enumów z definicji C++.
4. Graf typów zaczyna od wybranych RPC, `transaction`, `signed_transaction` i jawnego `object_structs`, następnie przechodzi zależności. Nie generuje automatycznie całej zawartości repozytorium chaina.
5. IR zawiera typy, kolejność pól, tagi, object spaces/type IDs, API discovery, parametry pozycyjne, opcjonalność, źródła i status wsparcia. **Oba zapisane dokumenty mają `schemaVersion: 1`**, mimo komentarzy nazywających model „v3”.
6. Rust generator emituje `mod.rs`, `ids.rs`, `types.rs`, `operations.rs`, `static_variants.rs`, `fc.rs`, `rpc.rs`. Są to typy protokołu, serde, serializacja FC, mechaniczne helpery i typowane parametry/dekodery RPC. Nie jest to generator scenariuszy biznesowych.
7. Ręcznie pisane `graphene-primitives`, `graphene-core`, `graphene-fc`, `graphene-transport` i `graphene-chain-swaplock-api` dostarczają runtime oraz API. `graphene` jest fasadą. API odpowiada za lookupy, opłaty, przygotowanie transakcji i subskrypcje.
8. Specyfikacje i wygenerowane źródła są commitowane. CI odtwarza je z przypiętych źródeł i sprawdza diff.

Źródła: [extractor](../open-graphene/crates/open-graphene-spec-gen/src/generate.rs), [IR](../open-graphene/crates/open-graphene-json-schema/src/lib.rs), [TypeRef](../open-graphene/crates/open-graphene-json-schema/src/types.rs), [generator Rust](../open-graphene/crates/open-graphene-gen-bindings-rs/src/generate/mod.rs), [CI](../.github/workflows/ci.yml), [rejestr decyzji](DECISIONS.md).

## 2. Zweryfikowany zakres i ograniczenia

| Zapisana specyfikacja | Structs* | Enumy | Static variants | Operacje | Object types | API / metody RPC |
|---|---:|---:|---:|---:|---:|---:|
| Swaplock | 277 | 11 | 13 | 95 | 49 | 5 / 47 |
| BitShares | 209 | 11 | 11 | 78 | 42 | 2 / 7 |

\* Liczby sekcji IR nie są rozłącznym liczeniem typów publicznych; struktury operacji występują również w `structs`. W obu chainach jest 7 operacji wirtualnych. Swaplock ma 22 object types z `structRef`, BitShares 4. Brak `structRef` nie oznacza braku danego obiektu w chainie.

Swaplock jest aktywnym pełnym SDK. BitShares służy jako drugi rzeczywisty profil generatora i zestaw lokalnych testów. ACTA i R-Squared mają placeholdery; „pełna parytetowość z Rust” nie oznacza tworzenia dla nich kompletnego SDK bez specyfikacji.

Aktualny Rust zawiera znacznie więcej niż pierwszy transfer: typowane database/history/orders/crypto, generyczny transaction builder, multisigning, rodziny operacji poniżej, memo, reconnect sesji, live dispatcher, subskrypcje rynku i ChainStore. `operations/api.rs` ma 88 publicznych metod, w tym `transaction` oraz `sign_transfer_with_wif`; sama liczba operacji IR nie dowodzi pokrycia ergonomicznymi builderami ani wszystkich wartości FC.

Istniejący `graphene-chain-swaplock-wasm` eksportuje tylko `serializeTransaction`, `transactionDigest`, `digestMatches`. Pozostaje osobnym narzędziem/wzorcem do porównań. Nowy runtime TS nie będzie wymagał WASM, Rust, Cargo ani źródeł C++ po stronie konsumenta.

Wykonana walidacja: `env -u SWAPLOCK_ACTIVE_WIF -u UPDATE_GOLDEN cargo test --workspace --offline --locked` zakończyło się kodem 0: 429 passed, 0 failed, 0 ignored w 34 podsumowaniach. Testy zależne od nieustawionego WIF mogą kończyć się wcześniej jako zaliczone — wynik nie dowodzi wykonania wszystkich fixture podpisów. Nie wykonano live RPC, broadcastów, builda przeglądarkowego, Clippy ani pełnej reprodukcji CI. Nie zmieniano źródeł ani wyników generatorów.

Przed analizą istniała lokalna zmiana `D open-graphene-packages/TODO_MISSING.md`; zachować ją. Nie odtwarzać tego pliku jako części planu.

## 3. Oryginalny SDK i C++ są wzorcami protokołu

Zweryfikowane lokalne checkouty: `bitsharesjs` @ `94846cee720f0b80ea2e2282a94eec78dbbf9016`, `bitsharesjs-ws` @ `67e79722e2971c491befe8ce52d90a4b12672db5`. Lokalny BitShares Core ma HEAD `fe7000cab0f71bb2e77699a771b21604d7c36a38`, a CI przypina `b92b82ba3e57381d111f28383e8cfa89a8356966`: fixture muszą zapisywać, z której rewizji pochodzą. Swaplock w `blockchains/` jest drzewem bez własnego `.git`; `git -C ... rev-parse` zwraca wtedy commit repo nadrzędnego i nie dowodzi rewizji C++. CI używa archiwum `50179db9950f7297fdb94145efa33a69fd5bbc5f` z kontrolą SHA-256.

| Zachowanie | Bezpośrednie źródło lokalne | Wniosek dla TS |
|---|---|---|
| FC i JSON | `bitsharesjs/lib/serializer/src/{serializer,types,operations}.js` | Porównywać bajty i JSON; definicje operacji generować z IR, nie kopiować ręcznie `operations.js`. |
| Typowane ID a generic object ID | `bitsharesjs/lib/chain/src/ObjectId.js`, `types.js`; BitShares Core `libraries/protocol/include/graphene/protocol/object_id.hpp` | Oddzielić instance-varint od pełnego packed-u64. |
| `extension<T>` | `types.js: Types.extension`; C++ i Rust extension renderery | Count + pary indeks/payload; inne niż seria optionali i niż set static variants. |
| Podpis i nonce | `bitsharesjs/lib/ecc/src/{signature,ecdsa}.js` | Historyczne reguły Graphene i retry nonce nie są tożsame z aktualnym Rust/Swaplock. |
| Memo/ECDH | `bitsharesjs/lib/ecc/src/{PrivateKey,aes}.js`, Rust `graphene-fc/src/{keys,memo}.rs` | Fixture wymiany między implementacjami, nie tylko round-trip TS. |
| Nagłówek, opłaty, podpis, submit | `bitsharesjs/lib/chain/src/TransactionBuilder.js` | TaPoS, fee recursion, hash unsigned transaction, callback confirmation. Politykę SDK opisać jawnie. |
| Dynamiczne API IDs | `bitsharesjs-ws/src/{GrapheneApi,ApiInstances}.js` | Login przez API 1, discovery nazw API, ponowne discovery po reconnect. |
| Callbacki | `bitsharesjs-ws/src/ChainWebSocket.js` | Funkcja JS zostaje lokalnie, na drut idzie callback ID; `notice.params = [id, payload]`. |
| Cache i historia | `bitsharesjs/lib/chain/src/ChainStore.js` | Jeden database callback, get_full_accounts, id-only deletion, reconciliation historii. |

Zachowania wygody oryginalnego JS nie wszystkie są kontraktem nowego SDK: JS sortuje set/map w serializerze, używa globalnego `Apis.instance()`, `Buffer`/`Long`, ręcznej listy operacji i zwykłego `JSON.parse`. TS zachowa wynik protokołu, ale będzie miał instancje klientów, `bigint`, `Uint8Array`, generowanie i jawne walidowanie kolejności.

**Wykryta rozbieżność, obowiązkowa bramka E0:** Rust `generate/fc.rs::render_fc_id_impls` generuje dla generic `ObjectId` wywołanie `write_protocol_object_id(..., None, None)`, które zapisuje tylko varint instancji. C++ odbija dla `object_id_type` pole `uint64_t number`; JS pakuje `(space << 56) | (type << 48) | instance` do u64 LE. Przykład `1.2.345`: generic ID daje `5901000000000201`, typowane account ID `d902`. Nie uznawać aktualnego Rust za oracle dla tego przypadku. Przed implementacją tej części potwierdzić właściwe źródło Swaplock, dodać fixture i albo naprawić wąski błąd Rust, albo jawnie oznaczyć rozbieżność i zablokować deklarowanie parytetu tego przypadku. Nie utrwalać błędu w TS.

Porządek rozstrzygania: źródła C++ właściwego chaina i rewizji -> niezależne wektory/oryginalny JS dla wspólnego protokołu -> aktualny Rust jako wzorzec zakresu produktu. Zielone testy jednej implementacji nie zastępują tego porównania.

## 4. Docelowa organizacja i granice

Generator TS będzie **narzędziem w Rust**, w `open-graphene/crates/open-graphene-gen-bindings-ts`, czytającym istniejący `Protocol`. To wykorzystuje aktualny IR, Cargo workspace i pipeline. Emitowany SDK/runtime jest natywnym TypeScriptem.

Na początku oba generatory czytają istniejące specyfikacje z `open-graphene-packages/rust/graphene-chain-*/graphene-chain-*-spec/dist/`. Nie tworzyć ich kopii w TS. Przeniesienie speców do katalogu neutralnego językowo można zrobić później jako osobną zmianę ścieżek; nie jest zależnością tego projektu.

```text
open-graphene/crates/
  open-graphene-codegen-common/       # małe wspólne reguły protokołu, E1
  open-graphene-gen-bindings-ts/      # CLI + renderery TS + testy
open-graphene-packages/typescript/
  package.json, pnpm-workspace.yaml, pnpm-lock.yaml, tsconfig.base.json
  graphene-primitives/
  graphene-codec/
  graphene-core/
  graphene-fc/
  graphene-transport/
  graphene/
  graphene-chain-swaplock/
    graphene-chain-swaplock-bindings/
    graphene-chain-swaplock-api/
  graphene-chain-bitshares/
    graphene-chain-bitshares-bindings/
  tests/fixtures/, tests/browser/, examples/, scripts/
```

| Pakiet / proponowana nazwa npm | Odpowiedzialność | Zależności wewnętrzne |
|---|---|---|
| `graphene-primitives` / `@open-graphene/primitives` | Branded ID, parse/validate, amount/reference values | brak |
| `graphene-codec` / `@open-graphene/codec` | JSON bez utraty precyzji, bazowe runtime codecs, błędy ze ścieżką pola | primitives |
| `graphene-fc` / `@open-graphene/fc` | FC writer, hashe, klucze, podpis, memo; subpath `signing` | primitives |
| `graphene-core` / `@open-graphene/core` | Kwoty decimal/raw, nagłówki, nazwy kont, salda, analiza authority | primitives |
| `graphene-transport` / `@open-graphene/transport` | WS, sesja, JSON-RPC, request/callback dispatch, lifecycle | codec; bez chain bindings |
| `graphene-chain-*-bindings` / `@open-graphene/chain-*-bindings` | Wygenerowane typy, JSON codecs, FC i deskryptory RPC | primitives, codec, fc |
| `graphene-chain-swaplock-api` / `@open-graphene/chain-swaplock-api` | Ręczne request builders, transakcje, typed subscriptions/cache | swaplock-bindings, core, transport; signing przez jawny adapter |
| `graphene` / `@open-graphene/graphene` | Fasada `Graphene`, chain config i eksporty | swaplock-api |

`codec` jest świadomym dodatkiem wobec Rust: TypeScript nie ma runtime odpowiednika serde. Wspólny codec zapobiega skopiowaniu parserów do transportu, core i każdego chaina. `core` pozostaje bez RPC, broadcastu i konstrukcji operacji konkretnego chaina.

Wspólny crate codegen ma obejmować wyłącznie walidację grafu, klasyfikację wire shapes i mały wersjonowany profil zgodności. Nie przenosić do niego formatterów, nazw Rust/TS, kodu SDK czy templating frameworka. Wprowadzać go przy pierwszej regule potrzebnej obu generatorom, z testem niezmienionego wyniku Rust poza osobno udokumentowanymi naprawami E0.

Ważne: IR nie opisuje dziś całej semantyki. Ma `ordering: unresolved`, pseudo-refy `config` i `required_fee`, extension structs bez oznaczenia sparse oraz chain-specific JSON wyjątki w Rust generatorze. Profil wspólny ma opisywać te konkretne wyjątki z dowodem i warunkami wsparcia. Docelowo fakty wydobywalne z C++ przenosić do extractora/IR. Nie tworzyć osobnego, ukrytego zestawu interpretacji w generatorze TS.

Utworzenie katalogów/pakietów w istniejącym repo nie jest rejestracją nowych repozytoriów. Jeżeli później będą potrzebne nowe checkouty, użyć właściwego `mirror.toml` i `mctl`.

## 5. Kontrakt TypeScript i generowania

**Toolchain:** osobny pnpm workspace w katalogu `typescript`, niezależny od nadrzędnego workspace Mirrorboards. pnpm 10, konkretna wersja przypięta przy E1; ESM, ES2022, Node >=22.12, współczesne przeglądarki. `tsc` emituje JS, declarations i source maps; `strict`, `noUncheckedIndexedAccess`, `exactOptionalPropertyTypes`. Biblioteki eksportują build z `dist`, jawne `exports` i `sideEffects: false`. Brak CJS i React hooks w zakresie parytetu. Wersje TypeScript/test runnera i zależności zostaną przypięte w lockfile w E1; nie instalowano ich podczas planowania.

CLI: `cargo run -p open-graphene-gen-bindings-ts -- --spec <spec.json> --out-dir <src/generated>`. Dodatkowe `--check` generuje do katalogu tymczasowego i sprawdza także dodane/usunięte pliki. Zła wersja schematu, kolizja nazw, powtórzone indeksy/tagi, brak znanego referowanego typu lub sprzeczny profil kończą generację błędem przed podmianą outputu. Dopuszczone dynamiczne pola są jawnie raportowane.

Output: `index.ts`, `ids.ts`, `types.ts`, `operations.ts`, `static-variants.ts`, `json.ts`, `fc.ts`, `rpc.ts`, `support.json`. Źródła TS są commitowane, `dist` jest wynikiem builda. Nagłówek/provenance zawiera chain, rzeczywistą wersję IR, hash spec i wersję profilu; bez czasu generacji i ścieżek absolutnych. Stabilna kolejność i formatter przypięty wersją. Generowanie nie usuwa ręcznych plików poza własnym katalogiem.

| IR / semantyka | Reprezentacja runtime TS | Zasada |
|---|---|---|
| u8/u16/u32/i32 | `number` | finite integer i kontrola zakresu |
| i64/u64/u128 | `bigint` | pełny zakres; nigdy pośrednio przez niedokładny `number` |
| unsigned varint | `bigint` jako wartość; tagi/count jako sprawdzone małe integer | nie mylić długości/tagu z dowolnym u64; limity źródłowe |
| object ID/public key/vote/time | branded string | walidacja formatu, zakresów i chain prefix; runtime ID z pełnym space/type |
| bytes/fixed bytes/signature | `Uint8Array` | hex na JSON wire, wymagana długość; podpis 65 B |
| struct | interface z nazwami pól C++ (`snake_case`) | FC według `field.index`; bez niezależnego ręcznego DTO |
| operation/static variant | unia tuple `readonly [literalTag, Payload]` | bez zgadywania tagów; ergonomic helpers `operation.transfer(...)` |
| vector/set | readonly array | set: sprawdzona kolejność i unikalność |
| map/flat_map/pair | readonly tuple arrays / para | żadnego JS object/Map jako domyślnego wire modelu |
| optional field | `field?: T` | brak/null dekodowane według pola; output pomija nieobecne optional fields |
| optional w tablicy/RPC | `T \| null`; pominięcie argumentu jako osobny stan | zachować sloty; trailing omission nie jest null |
| enum | `as const` + typ liczbowy/unia, bitfield tam gdzie IR | wartości z C++, nie pozycje listy; sprawdzanie szerokości |
| AnyJson | `unknown`, przechowany lossless wire value | jawny powód w raporcie; brak FC |
| Unsupported | jawny brak capability / błąd | bez `any`, fikcyjnego typu i pomyślnej serializacji |

Interfejs bazowy `Codec<T>`: `decode(value: unknown): T`, `encode(value: T): WireValue`; osobno `parseJson(text)` / `stringifyJson(value)` i `toFcBytes(value)`. Decoder jest rzeczywistą walidacją, nie `JSON.parse(...) as T`. Generowane funkcje nie wykonują I/O. Rekurencyjne typy, np. proposal/restriction, korzystają z funkcji/lazy odwołań, bez zależności od kolejności inicjalizacji modułów.

`WireValue` dopuszcza niepubliczny szczegół reprezentacji liczby bezstratnej; konsument dostaje normalne wartości domenowe. `graphene-codec` opakuje `lossless-json`, aby zachować także JSON numeric tokens większe od 2^53 i umieć je wysłać zgodnie z profilem. String decimal i numeric token pozostają rozróżnione. `json: decimal_string` ma pierwszeństwo; brak hintu dla i64/u64 zachowuje aktualny JSON output Rust przez dokładny token liczbowy. Decoder przyjmuje także poprawny decimal string od noda. Nie stosować globalnego `BigInt.prototype.toJSON`; nie zmieniać wszystkich dużych liczb na string bez kontraktu. Odrzucać unsafe `number`, którego precyzja już została utracona przed wywołaniem codec.

`get_objects` emituje tagged `ProtocolObject` dla modeli z `structRef` oraz jawną gałąź `unknown` dla pozostałych. Znany type ID z niepoprawnym payloadem jest błędem, nie przechodzi do unknown. Typowane gettery sprawdzają dokładne requested ID; wyniki zachowują kolejność, liczbę slotów i null. Unknown nie nadaje się do FC ani podpisywania.

Deskryptor RPC zawiera wire API/method, codec parametrów/return, indeksy i zasady required/nullable/default. Parametry usuwać wyłącznie z końca; luki w środku odrzucać, chyba że jawna nullable pozycja pozwala wysłać null. Nie wykonywać tekstu C++ z `defaultValue`. CamelCase stosować w ergonomicznym API TS, literalne nazwy wire zachować w deskryptorach.

## 6. FC, podpisy i transakcje — warunki poprawności

- Typowane ID: instance varint z kontrolą oczekiwanego space/type i zakresu ze źródeł C++. Generic object ID: odrębny packed-u64 LE, po bramce E0.
- Integers LE, UTF-8 length w bajtach, length-prefixed bytes, fixed bytes bez dodatkowej długości, seconds jako u32 UTC. Nie używać bitwise JS do operacji na u64.
- `extension<T>`: count oraz indeks/payload obecnych członków. `extensions_type`: set static variants. Zachować aktualne ograniczenie pustych generic extension payloadów, dopóki nie ma wektorów potwierdzających rozszerzenie wsparcia.
- Portować wspierane niepuste wyjątki: Content Card `expected_hash`, Data Room `write_policy`, `expected_access_state`. Pusta access extension zachowuje legacy JSON `[]`, niepusta obiekt. Legacy brak write policy nie może włączyć strict writes.
- Porządek flat_set/flat_map: numeric, instance, decoded public key bytes, variant tag etc. zgodnie z potwierdzoną regułą. Bez automatycznego sortowania w serializerze. Opcjonalne jawne helpery normalizacji działają na kopii poza nim. `ordering: unresolved` wymaga reguły profilu z fixture lub błędu.
- Memo jest już wspierane w aktualnym Rust. Zachować optional byte marker, compressed public keys, nonce u64, length-prefixed encrypted message. `Address` w niepustym authority map pozostaje unsupported, choć runtime ma osobny typ Address.
- Wszystkie 95 operacji mogą mieć typ JSON; możliwość FC/sign/broadcast musi być oceniana osobno. Operacje wirtualne są czytelne w historii, lecz nie mogą wejść do podpisywanej transakcji. Nested unsupported payload ma zatrzymać całe podpisywanie.
- `signaturePreimage = chainIdBytes[32] || unsignedTransactionFcBytes`; `digest = SHA256(preimage)`. Chain ID jest jawnym argumentem, walidowanym z konfiguracją/sesją; generator może emitować default, ale nie może podpisywać dla innego chaina przez zaszytą stałą. Podpisy nie wchodzą do unsigned preimage.
- Compact signature: `header = 31 + recoveryId`, następnie r i s po 32 B. Aktualny Rust sprawdza header i low-S; historyczny `bitsharesjs` dodatkowo wymaga kanonicznych długości r/s i ponawia nonce. Modelować jawny profil `swaplock-low-s` oraz `graphene-legacy`; nie wywodzić go z samego prefixu `BTS`.
- Dla Swaplock zgodność podpisu sprawdzać z aktualnymi publicznymi fixture Rust. Dla legacy BitShares wymagać prawidłowego recovery/verification i kanoniczności; identyczne signature bytes tylko dla ustalonego algorytmu nonce. Nie wymagać jednakowych podpisów od różnych poprawnych strategii nonce. Nie przenosić starych sekretnych fixture wymagających `.env`.
- Crypto adapter: `@noble/curves` secp256k1, `@noble/hashes`, `@noble/ciphers` AES-CBC oraz `@scure/base` base58. Sprawdzić konkretne API w przypiętych wersjach; szczególnie wyłączyć ponowne hashowanie przy podpisywaniu gotowego digestu. Nie pisać własnych krzywych/kryptografii.
- Memo: ECDH shared secret = SHA512(x coordinate); seed = nonce decimal + hex(shared secret); SHA512 seed daje AES key/IV; checksum plaintext zgodnie z Rust/JS. Sprawdzić null key oddzielnie: poprawny null memo key w serializacji nie jest kluczem dopuszczalnym do ECDH.
- Publiczny podpis przez `Signer.signDigest(...)` oraz opcjonalny adapter WIF, także multi-key i external components/recovery. Przyjmować tylko potrzebne unikalne podpisy. Prywatne dane nie trafiają do błędów, `toJSON`, logów ani notice dumpów.
- `PreparedTransaction` utrwala defensywną kopię danych oraz bajty/digest użyte do podpisu; mutacja zewnętrznego obiektu po prepare nie może zmienić broadcastowanego payloadu. W TS samo `readonly` nie zabezpiecza runtime.

## 7. Transport i ergonomiczne API

Jeden asynchroniczny dispatcher WS obsługuje requesty, jednorazowe callbacki i subskrypcje. TS nie potrzebuje kopiowania historycznego podziału Rust na osobny blocking/live API. Fasada zachowuje znaczenie wywołań, używając Promise, AbortSignal i AsyncIterable.

Kontrakt planowanego API:

```ts
const client = await Graphene.swaplock()
  .servers([rpcUrl])
  .chainId(expectedChainId)
  .connect();

const account = await client.database().accountById("1.2.100").get();
const subscription = await client.database().accountById("1.2.100").subscribe();
const initial = subscription.initial;
for await (const update of subscription) {
  // typed update; iterator.return()/unsubscribe() zwalnia lokalną subskrypcję
}

const prepared = await client.operations().transfer()
  .from("alice").to("bob").amountDecimal("1.25", "BTS")
  .maxFeeRaw(1000n).prepare();
const signed = await prepared.sign(signer);
const confirmation = await client.networkBroadcast().broadcastWithCallback(signed);
await client.close();
```

To deklaracja docelowego kształtu, nie kod już istniejący. Przykłady get/subscribe i prepare/sign/broadcast będą osobnymi uruchamialnymi plikami, jak w Rust.

Transport:

1. Native global WebSocket z możliwością wstrzyknięcia factory; brak `node:*` i `Buffer` w domyślnym browser graph. Client construction nie otwiera połączenia podczas importu/SSR.
2. Login i discovery API IDs, database wymagane, pozostałe API opcjonalne. IR obecnie oznacza wszystkie API jako required; profil sesji musi to skorygować zgodnie z istniejącym runtime. Wybrana jawnie funkcja wymaga właściwego API i zwraca `MissingApi`.
3. FirstAvailable / LowestLatency, sprawdzanie chain ID również po reconnect. Każda próba ma timeout i cleanup.
4. Oddzielne mapy pending requests i callbacków, rejestracja przed send, numery/stringi ID normalizowane bez utraty precyzji. Obsługa notice przed odpowiedzią requestu, późnych odpowiedzi, błędów RPC, close i timeout.
5. Abort/timeout czyści mapy i listenery. Stare odpowiedzi z poprzedniej sesji nie mogą rozwiązać requestu nowej sesji. Ograniczona kolejka subskrypcji; overflow kończy ją jawnym błędem wymagającym resync, bez cichego gubienia historii.
6. Jeden `set_subscribe_callback` na sesję database; lokalny multicast. Odpięcie jednego obserwatora nie wyłącza innych. Market callback ma własną rejestrację i odpowiadający jej unsubscribe.
7. Reconnect i retry tylko dla jawnie idempotentnych odczytów. Nigdy automatycznie ponawiać broadcastu o nieznanym wyniku. Retry nie może ponownie użyć starych API IDs.
8. Parytet Rust: reconnect sesji, ale brak obietnicy automatycznego resubscribe/ciągłości historii. Utrata połączenia kończy aktywne streamy; ChainStore oznacza dane jako stale i daje jawny resync. Automatyczna ciągłość to osobny przyszły zakres.
9. Submit-only i callback-confirmed broadcast są osobnymi metodami. Confirmation oznacza inclusion/applied block zgodnie z callbackiem, nie nieodwracalność chaina.

High-level:

- Database zawsze konsumuje generated codecs. `get_account_balances` daje Asset[], a balance subscription używa `get_full_accounts` / AccountBalanceObject; pusta lista assetów oznacza wszystkie. Orders: initial full_account.limit_orders + aktualizacje/deletions po ID i sellerze.
- History zachowuje obecne `limit + offset`, overscan/lookahead i internal operation ID reconciliation; offset nie gwarantuje stabilnego snapshotu przy nowych wpisach. Nie odtwarzać porzuconego publicznego cursor API z D111.
- Initial snapshot + notices: listener przed requestem snapshotu, buforowanie na czas inicjalizacji i kontrolowane reconciliation. Testować aktualizacje/deletions przychodzące podczas bootstrapu.
- ChainStore per client, explicit watched IDs, generowane ProtocolObject i unknown branch; nie globalny cache wszystkich kont jak stary JS.
- Przygotowanie transakcji: rozwiązanie kont/assetów, precyzja, header z chain head, required fees, liczba wyników zgodna z liczbą operacji, fee ceilings i balance checks tam gdzie istnieją w danym builderze. Proposals wymagają zachowania rekurencyjnego fee tree; nie wystarczy zip i odrzucenie nested fees. Bez niejawnego fee-asset fallbacku z dawnego JS.
- Data Rooms/Content Cards: pełny zakres aktualnego Rust, łącznie z member account/public-key union, permissions, epochs, write policy, expected_hash oraz RoomAccessPrecondition. Snapshot digest nie sortuje members i jest związany z konkretnym pokojem; oddzielne snapshots dla removal i rotation. SDK nie przejmuje zarządzania vault/key storage.

## 8. Etapy wykonania i odbiór

Każdy etap kończy się działającym, reviewowalnym zakresem i zaktualizowaną macierzą wsparcia. Nie wolno zakończyć całej pracy na etapie „bindingi kompilują się”.

| Etap | Konkretne zadania | Warunek odbioru |
|---|---|---|
| **E0: źródła i wektory** | Zamrozić inwentarz Rust; spisać wyjątki generatora; fixture publiczne z Rust, C++ i przypiętego JS; rozstrzygnąć generic object ID oraz profile podpisu. | Każda rozbieżność ma źródło, test i rozstrzygnięcie albo jawne zablokowanie danego capability. Nie deklarować całego parytetu z otwartym błędem. |
| **E1: fundament** | pnpm workspace, ESM/declarations, primitives, codec, Rust CLI TS; wąskie codegen-common przy pierwszym współdzielonym wyjątku; initial CI. | Instalacja/build z katalogu TS, Node i browser import, exact numeric JSON; brak zmiany wire Rust poza naprawami E0. |
| **E2: pełne typy i JSON** | Wygenerować oba chainy, wszystkie typy/operacje/static variants, runtime codecs, object routing, RPC descriptors, support report. | Swaplock i BitShares typecheck; compile-time negative cases i runtime malformed fixtures; deterministic regen/check. Brak ręcznych kopii protocol interfaces w API. |
| **E3: FC i lokalne crypto** | Primitive writer, generowane FC per support matrix, podpisy, klucze/WIF, brainkey/login, memo, access digest. | Byte-for-byte transaction/preimage/digest/extension fixtures; signature verification/recovery i właściwy profil; unsupported paths odrzucane także w nested operations. Node i browser. |
| **E4: sesja i odczyty** | WS dispatcher, discovery, retry/timeout/abort; wygenerowane RPC w high-level database/history/orders/crypto; fasada. | Deterministyczny lokalny WS harness: out-of-order, optional API, chain mismatch, reconnect, slot/null semantics, exact integers. Przykłady read-only. |
| **E5: transakcje** | Generyczny multi-op builder, transfer vertical slice, prepare/sign/multisign, submit-only i callback confirmation, fee tree. | Publiczne fixture + local WS fake, maksymalne opłaty, virtual-op reject, unknown broadcast outcome bez retry; mutation-after-prepare test. |
| **E6: reszta API** | Wszystkie obecne rodziny builderów Rust; Data Rooms/Content Cards, authority oraz plugin RPC. | Każda rodzina ma reprezentatywną pozytywną i istotne negatywne ścieżki, z macierzą właściwości/ograniczeń. Convenience nie maskuje ograniczeń FC. |
| **E7: live i cache** | Account/asset/DGP/balances/orders/history/market subscriptions, ChainStore, bootstrap race handling, cleanup i stale/resync. | Kilka subskrypcji + requesty + callback na jednej sesji; rozłączenie, kolejność notice/reply, deletion, slow consumer, brak wycieku listenerów i stale state udającego aktualne. |
| **E8: dostarczenie** | Pakowanie, docs, minimalne przykłady, pełny CI/regeneration, offline parity report; osobno opt-in live verification. | Konsument instaluje spakowane paczki poza monorepo, działa Node i browser, brak Rust/WASM runtime, odtworzenie generacji z clean checkoutu; zero nieopisanych luk wobec ustalonego Rust baseline. |

Zależności: E0 -> E1 -> E2; E3 i E4 wykorzystują E2; E5 potrzebuje obu; E6 po E5; E7 potrzebuje E4/E5; E8 zamyka całość. Ten graf nie jest poleceniem uruchamiania dodatkowych agentów.

Inwentarz do E6 (nazwy grup odpowiadają modułom Rust):

| Grupa | Zakres |
|---|---|
| transfer / transaction | transfer, generyczne multi-op, signing z jednym/wieloma kluczami |
| account | create, update, upgrade, whitelist, transfer |
| asset / asset_admin | issue, reserve, update, create, issuer, fee pool, claim fees/pool, settle/global settle, feed producers/feed, override, bitasset |
| limit_order / call_order | create/update/cancel, call update, bid collateral |
| htlc / proposal | create/redeem/extend; create/update/delete proposals |
| liquidity_pool / samet_fund | create/delete/deposit/withdraw/exchange/update; create/update/delete/borrow/repay |
| credit_offer | create/update/delete/accept, deal repay/update |
| withdraw_permission / vesting / ticket | wszystkie obecne builders tych trzech modułów |
| governance / custom_authority | committee, witness, worker, custom, global parameters, custom authority create/update/delete |
| assert / balance_claim / blind | assert, balance claim, to-blind/blind/from-blind |
| data_room / room_access | create/update/delete, add/update/remove member, rotate key, preconditions/snapshot |
| content_card | create/update/remove, grant create/revoke, link create/update/remove |

Raw binding coverage wynika z IR, nie z tej tabeli. Operacje bez ergonomic buildera w Rust są dostępne przez generated operation + generyczny transaction builder, z tymi samymi warunkami FC.

## 9. Macierz testów i CI

- **Generator:** fixture małego chaina, oba pełne specy, collision/reserved identifiers, duplikaty indeksów/tagów, dangling refs, schema version, zero timestamps, cycles. Test `--check` wykrywa także stare/usunięte output files. Nie zmieniać generated sources ręcznie.
- **Liczby/JSON:** i64 min/max, u64 max, u128 JSON, 2^53±1, u32 >2^31, liczby jako token/string, nested arrays/maps, błędny input number, missing/null/false/0, bytes/hex i długości, enum/variant mismatch, unknown object, ID mismatch.
- **FC cross-language:** transfer z/bez memo, authority maps, unsorted/duplicate keys, typed/generic ID, vote ID, HTLC fixed hashes, fee parameters, proposal i recursive restrictions, empty/nonempty extensions, access snapshots. Wektory mają zapisane źródło/rewizję, nie wymagają sekretów ani noda.
- **Podpisy:** header/range/low-S/legacy, recovery, wrong key, wrong chain ID, double hashing, multi-sign dedup; publiczny deterministyczny private-key fixture. Znane signed bytes można sprawdzać bez posiadania prywatnego WIF ich historycznego autora.
- **Runtime:** MockWebSocket do deterministycznego dispatch i lokalny WS server do integracji. Nie wystarczy mocked `call()` omijający rzeczywiste serializowanie JSON i discovery.
- **Browser:** te same wektory FC/crypto, odczyt/subskrypcja przez lokalny WS, import gotowych paczek bez Buffer/polyfill/node builtin, wyłączenie import side effects. Test runner można oprzeć na Vitest + Playwright; przypiąć wersje w E1.
- **Pakowanie:** `pnpm pack`, instalacja artefaktów w oddzielnym katalogu Node i fixture browser bundlera; test JS + `.d.ts` oraz package exports, tree shaking bez eager signing importu.
- **CI:** zachować rust fmt/clippy/test; dodać TS build/typecheck/test/browser/pack. Job regen używa obecnego archiwum Swaplock + pin BitShares, regeneruje spec raz, potem Rust i TS. Gate sprawdza oba drzewa bindings oraz schema/profile output, włącznie z nowymi plikami.
- **Live:** dodatkowe, jawnie uruchamiane testy z podanym endpointem/chain ID; read-only i write smoke rozdzielone. Publikacja npm, broadcasty i deploy nie należą do tej fazy planowania i nie wynikają automatycznie z E8.

`support.json` ma rozdzielić: typ istnieje, JSON decode/encode, FC support (również warunkowe, np. empty-only), signing eligibility, metoda RPC, high-level builder, live subscription. Dołączyć dependency path/reason dla unsupported. Nie oznaczać całej operacji jako supported tylko dlatego, że TS interface się skompilował.

## 10. Kolejność startu po zmianie modelu

Pierwsze zadanie implementacyjne to **wyłącznie E0**: odczytać ten plan, sprawdzić aktualny HEAD/working tree, zamrozić macierz zakresu i przygotować publiczne wektory rozstrzygające object ID, JSON integers, sparse extensions i podpisy. Następnie rozpocząć E1. Każdy etap raportuje zmienione pliki, wykonane testy i otwarte różnice; aktualizacje zakresu odnotowywać w tym planie. Rejestr `DECISIONS.md` jest append-only, a historyczne decyzje trzeba czytać wraz z późniejszymi zmianami i aktualnym kodem.

Najważniejsze pułapki dla modelu wykonawczego: nie kopiować nieaktualnych opisów memo/reconnect/ChainStore, nie pomylić komentarza „v3” z `schemaVersion`, nie traktować pustych ACTA/R-Squared jako działających SDK, nie implementować od nowa parsera C++, nie kopiować ręcznie JS operation table, nie redukować zakresu do `.d.ts`, nie przejmować polityki fee/sign/broadcast do generatora, nie używać starej reguły podpisu dla Swaplock bez profilu, nie traktować zgodności z Rust jako dowodu przy wykrytej sprzeczności z C++.

## 11. Zweryfikowane zewnętrzne API do wyborów runtime

To źródła dotyczące bibliotek/platformy; fakty protokołu powyżej pochodzą z lokalnego C++/Rust/JS. Konkretne wersje zależności zostaną utrwalone dopiero w E1.

- [lossless-json](https://github.com/josdejong/lossless-json): parse/stringify zachowujące numeric tokens, obsługa Node i przeglądarki, typy TS.
- [noble-curves](https://github.com/paulmillr/noble-curves#prehashed-signing): secp256k1 i jawny tryb `prehash: false` dla gotowego digestu; dokumentacja opisuje recovery i extra entropy.
- [noble-hashes](https://github.com/paulmillr/noble-hashes), [noble-ciphers](https://github.com/paulmillr/noble-ciphers), [scure-base](https://github.com/paulmillr/scure-base): kandydaci runtime dla hashy, AES-CBC i base58.
- [Node WebSocket](https://nodejs.org/api/globals.html#class-websocket): globalny browser-compatible client; stabilny od Node 22.4.
- [TypeScript exactOptionalPropertyTypes](https://www.typescriptlang.org/tsconfig/exactOptionalPropertyTypes.html): rozróżnienie nieobecnego pola od przypisania undefined, istotne dla param omission i optional fields.
