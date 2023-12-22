// :33333 =^-^= :)) :D
#define WIN32_LEAN_AND_MEAN 1
#define NOMINMAX 1
#include <Windows.h>
#include <Psapi.h>

#include <iostream>
#include <string>
#include <vector>
#include <algorithm>
#include <stdexcept>
#include <utility>


template<typename TKey, typename TValue, int IUnk>
struct CHashMap {
public:
    int m_curSize;
    int m_numUsed;
    int m_curMask;
    int m_growThreshold;

    using Hash = unsigned;

    struct Element {
        TValue v;
        TKey k;
        Hash h;
    };

    Element* m_elements;
};

using CHashMapVars = CHashMap<const char*, int, 7>;
using ElVars = CHashMapVars::Element;
constexpr auto wtf = offsetof(CHashMapVars, m_elements);
constexpr auto wtf2 = offsetof(ElVars, h);

using namespace std;

static wstring utf8towide(const string& i) {
    auto sizeinchars = MultiByteToWideChar(
        CP_UTF8,
        MB_ERR_INVALID_CHARS,
        i.c_str(),
        static_cast<int>(i.size()),
        nullptr,
        0
    );

    wstring result;
    if (sizeinchars <= 0)
        return result;

    result.resize(static_cast<size_t>(sizeinchars));
    sizeinchars = MultiByteToWideChar(
        CP_UTF8,
        MB_ERR_INVALID_CHARS,
        i.c_str(),
        static_cast<int>(i.size()),
        result.data(),
        sizeinchars
    );
    // ignore the error, it's very unlikely to fail
    return result;
}

static string widetoutf8(const wstring& i) {
    auto sizeinbytes = WideCharToMultiByte(
        CP_UTF8,
        WC_ERR_INVALID_CHARS,
        i.c_str(),
        static_cast<int>(i.size()),
        nullptr,
        0,
        nullptr,
        nullptr
    );

    string result;
    if (sizeinbytes <= 0)
        return result;

    result.resize(static_cast<size_t>(sizeinbytes));
    sizeinbytes = WideCharToMultiByte(
        CP_UTF8,
        WC_ERR_INVALID_CHARS,
        i.c_str(),
        static_cast<int>(i.size()),
        result.data(),
        sizeinbytes,
        nullptr,
        nullptr
    );
    // ignore the error, it's very unlikely to fail
    return result;
}

static string getlasterrormsg(DWORD lasterror) {
    wstring sresult;
    LPWSTR buffptr = nullptr;
    auto result = FormatMessageW(
        FORMAT_MESSAGE_ALLOCATE_BUFFER | FORMAT_MESSAGE_FROM_SYSTEM | FORMAT_MESSAGE_IGNORE_INSERTS,
        nullptr,
        lasterror,
        0,
        reinterpret_cast<LPWSTR>(&buffptr),
        0,
        nullptr
    );
    DWORD lasterr = GetLastError();
    if (result == 0)
        return "getlasterrormsg failed with " + to_string(lasterr) + ", oh the irony";
    sresult = buffptr;
    LocalFree(buffptr); buffptr = nullptr;
    return widetoutf8(sresult);
}

static int diewitherror(const string& msg, DWORD lasterror) {
    cerr
        << "error: "
        << msg << " " << getlasterrormsg(lasterror)
        << " raw = 0x" << hex << lasterror << dec
        << endl;
    return EXIT_FAILURE;
}

static LPBYTE fastcodesearch(HANDLE proc, const MODULEINFO& modinfo, const vector<uint8_t>& contents) {
    DWORD themask = PAGE_EXECUTE_READ;
    MEMORY_BASIC_INFORMATION mbi{};
    SIZE_T readbytes = 0;
    auto address_low = reinterpret_cast<LPBYTE>(modinfo.lpBaseOfDll);
    auto address_high = address_low + modinfo.SizeOfImage;

    auto iterbegin = contents.cbegin();
    auto iterend = contents.cend();

    while (address_low < address_high && VirtualQueryEx(proc, address_low, &mbi, sizeof(mbi)) != 0) {
        if ((mbi.State == MEM_COMMIT) && (mbi.Protect & themask) && !(mbi.Protect & PAGE_GUARD)) {
            size_t regsize = mbi.RegionSize;
            unique_ptr<uint8_t[]> region = make_unique<uint8_t[]>(regsize);
            auto mbeg = region.get();
            auto mend = region.get() + regsize;

            ReadProcessMemory(proc, mbi.BaseAddress, mbeg, regsize, &readbytes);
            auto mres{ search(mbeg, mend, iterbegin, iterend,
                [](uint8_t a, uint8_t b) {
                    return b == a || b == '?';
                })
            };

            if (mres != mend) {
                return mres - mbeg + reinterpret_cast<LPBYTE>(mbi.BaseAddress);
            }
        }

        address_low += mbi.RegionSize;
    }

    return nullptr;
}

template<typename T>
class defer {
    const T& thing;

public:
    inline defer(const T& t_) : thing(t_) { }
    // disallow copy and move
    defer(const defer& other) = delete;
    defer(defer&& other) = delete;
    // cast result to void so that you can return anything
    // from a lambda and it won't matter at all
    inline ~defer() { (void)thing(); }
};

// https://github.com/jwerle/murmurhash.c/blob/master/murmurhash.c
uint32_t
murmurhash(const char* key, uint32_t len, uint32_t seed) {
    uint32_t c1 = 0xcc9e2d51;
    uint32_t c2 = 0x1b873593;
    uint32_t r1 = 15;
    uint32_t r2 = 13;
    uint32_t m = 5;
    uint32_t n = 0xe6546b64;
    uint32_t h = 0;
    uint32_t k = 0;
    uint8_t* d = (uint8_t*)key; // 32 bit extract from `key'
    const uint32_t* chunks = NULL;
    const uint8_t* tail = NULL; // tail - last 8 bytes
    int i = 0;
    int l = len / 4; // chunk length

    h = seed;

    chunks = (const uint32_t*)(d + l * 4); // body
    tail = (const uint8_t*)(d + l * 4); // last 8 byte chunk of `key'

    // for each 4 byte chunk of `key'
    for (i = -l; i != 0; ++i) {
        // next 4 byte chunk of `key'
        k = chunks[i];

        // encode next 4 byte chunk of `key'
        k *= c1;
        k = (k << r1) | (k >> (32 - r1));
        k *= c2;

        // append to hash
        h ^= k;
        h = (h << r2) | (h >> (32 - r2));
        h = h * m + n;
    }

    k = 0;

    // remainder
    switch (len & 3) { // `len % 4'
    case 3: k ^= (tail[2] << 16);
    case 2: k ^= (tail[1] << 8);

    case 1:
        k ^= tail[0];
        k *= c1;
        k = (k << r1) | (k >> (32 - r1));
        k *= c2;
        h ^= k;
    }

    h ^= len;

    h ^= (h >> 16);
    h *= 0x85ebca6b;
    h ^= (h >> 13);
    h *= 0xc2b2ae35;
    h ^= (h >> 16);

    return h;
}

static uint32_t hashvarname(const string& n) {
    // GameMaker is using 0 as the seed.
    return murmurhash(n.c_str(), static_cast<uint32_t>(n.size()), 0);
}

static uint32_t hashvarslot(int varslot) {
    return (varslot * 0x9E3779B1) + 1;
}

struct smallstr {
    char c[0xff];
};

class reader {
    HANDLE prochandle;

public:
    reader(HANDLE ph)
        : prochandle(ph) {}

    template<typename T>
    T read(LPVOID rva) {
        T val{};
        SIZE_T readbytes = 0;
        if (!ReadProcessMemory(prochandle, rva, &val, sizeof(val), &readbytes))
            throw runtime_error("ReadProcessMemory failed horribly");
        return val;
    }

    template<typename T>
    void write(LPVOID rva, T val) {
        SIZE_T wrotebytes = 0;
        if (!WriteProcessMemory(prochandle, rva, &val, sizeof(val), &wrotebytes))
            throw runtime_error("WriteProcessMemory failed");
    }
};

int getvarslot(reader& r, LPBYTE instancevarlookup, const string& name) {
    auto hash = hashvarname(name);
    auto hashmask = r.read<uint32_t>(instancevarlookup + 8); // m_curMask
    auto idealpos = static_cast<int>(hashmask & hash & 0x7fffffffU);
    auto elements = r.read<LPBYTE>(instancevarlookup + 16); // m_pElements
    auto offhash = 16; // .h
    auto offk = 8; // .k
    auto offv = 0; // .v
    auto elsize = 24; // sizeof(Element)
    auto cursize = r.read<uint32_t>(instancevarlookup + 0); // m_numUsed

    auto curhash = r.read<uint32_t>(elements + (idealpos * elsize) + offhash);
    if (curhash != 0) {
        int i = -1;
        do {
            if (curhash == (hash & 0x7fffffffU)) {
                auto key = r.read<smallstr>(r.read<LPBYTE>(elements + (idealpos * elsize) + offk));
                if (strcmp(key.c, name.c_str()) == 0) {
                    return r.read<int>(elements + (idealpos * elsize) + offv);
                }
            }
            ++i;
            //if ((int)((pMap->m_curSize + uIdealPos) - (curHash & uMask) & uMask) < iAddr)
            if (static_cast<int>((cursize + idealpos) - (curhash & hashmask) & hashmask) < i) {
                return -1;
            }
            idealpos = (idealpos + 1) & hashmask;
            curhash = r.read<uint32_t>(elements + (idealpos * elsize) + offhash);
        } while (curhash != 0);
    }
    return -1;
}

LPBYTE getvarbyslot(reader& r, LPBYTE yyvars, int slot) {
    auto hash = hashvarslot(slot);
    auto hashmask = r.read<uint32_t>(yyvars + 8); // m_curMask
    auto idealpos = static_cast<int>(hashmask & hash & 0x7fffffffU);
    auto elements = r.read<LPBYTE>(yyvars + 16); // m_pElements
    auto offhash = 12; // .h
    auto offk = 8; // .k
    auto offv = 0; // .v
    auto elsize = 16; // sizeof(Element)
    auto cursize = r.read<uint32_t>(yyvars + 0); // m_numUsed

    auto curhash = r.read<uint32_t>(elements + (idealpos * elsize) + offhash);
    if (curhash != 0) {
        int i = -1;
        do {
            if (curhash == (hash & 0x7fffffffU)) {
                auto key = r.read<int>(elements + (idealpos * elsize) + offk);
                if (key == slot) {
                    return r.read<LPBYTE>(elements + (idealpos * elsize) + offv);
                }
            }
            ++i;
            if (static_cast<int>((cursize + idealpos) - (curhash & hashmask) & hashmask) < i) {
                return nullptr;
            }
            idealpos = (idealpos + 1) & hashmask;
            curhash = r.read<uint32_t>(elements + (idealpos * elsize) + offhash);
        } while (curhash != 0);
    }
    return nullptr;
}

template<typename T>
LPBYTE instanceloop(reader& r, LPBYTE runroom, const T& thing) {
    auto runroomlinkedlistoffs = 216;
    auto pnextptroffs = 0x198;

    auto curptr = r.read<LPBYTE>(runroom + runroomlinkedlistoffs);
    for (int idx = 0; curptr; ++idx) {
        if (thing(curptr, idx)) {
            return curptr;
        }

        curptr = r.read<LPBYTE>(curptr + pnextptroffs);
    }

    return nullptr;
}

int wmain(int argc, wchar_t *argv[]) {
    // fix console output
    SetConsoleOutputCP(CP_UTF8);
    SetConsoleCP(CP_UTF8);
    cout << "haiiiiiiiiiiiiiiiii :33333333333" << endl;

    DWORD procid = 0;
    DWORD lasterr = ERROR_SUCCESS;
    if (argc > 1) {
        procid = stoul(argv[1]);
    }
    else {
        cout << "plz type process id (in decimal): ";
        cin >> procid;
    }

    cout << "opening process id " << procid << endl;
    auto prochandle = OpenProcess(PROCESS_ALL_ACCESS, FALSE, procid);
    lasterr = GetLastError();
    defer prochandledefer{ [&] { CloseHandle(prochandle); } };
    if (!prochandle)
        return diewitherror("OpenProcess failed with", lasterr);

    MODULEINFO procmodinfo{};
    {
        HMODULE modules[2];
        DWORD needed = 0;
        auto enumres = EnumProcessModules(prochandle, modules, sizeof(modules), &needed);
        lasterr = GetLastError();
        if (!enumres)
            return diewitherror("EnumProcessModules failed with", lasterr);
        // [0] is always the main module, usually.....
        auto modgetres = GetModuleInformation(prochandle, modules[0], &procmodinfo, sizeof(procmodinfo));
        lasterr = GetLastError();
        if (!modgetres)
            return diewitherror("GetModuleInformation failed with", lasterr);
    }

    // now we have a process handle and a module struct
    // this means we can do sigscans now!
    /*
    auto builtinlookupsig = fastcodesearch(
        prochandle, procmodinfo,
        {
            // 48 89 5c 24 08 57 48 83 ec 20 48 8b 3d ? ? ? ? e8 ? ? ? ? 44 8b 4f 08 44 8b d8
            0x48, 0x89, 0x5c, 0x24,
            0x08, 0x57, 0x48, 0x83,
            0xec, 0x20, 0x48, 0x8b,
            0x3d,  '?',  '?',  '?', // here is the RVA
             '?', 0xe8,  '?',  '?',
             '?',  '?', 0x44, 0x8b,
            0x4f, 0x08, 0x44, 0x8b,
            0xd8
        }
    );*/
    auto varlookupsig = fastcodesearch(
        prochandle, procmodinfo, {
            // 48 83 ec 28 48 8b 0d ? ? ? ? e8 ? ? ? ? 48 85 c0 74 07 8b 00 48 83 c4 28 c3 b8 ff ff ff ff 48 83 c4 28 c3
            0x48, 0x83, 0xec, 0x28,
            0x48, 0x8b, 0x0d,  '?', // hashmap ptr
             '?',  '?',  '?', 0xe8,
             '?',  '?',  '?',  '?',
            0x48, 0x85, 0xc0, 0x74,
            0x07, 0x8b, 0x00, 0x48,
            0x83, 0xc4, 0x28, 0xc3,
            0xb8, 0xff, 0xff, 0xff,
            0xff, 0x48, 0x83, 0xc4,
            0x28, 0xc3 //, 0xcc, 0xcc
        }
    );

    auto runroomsig = fastcodesearch(
        prochandle, procmodinfo, {
            // 48 b8 00 00 00 00 00 00 10 c0 41 c7 40 0c 00 00 00 00 49 89 00 48 8b 05 ? ? ? ? 48 85 c0 74 48 85 d2
            0x48, 0xb8, 0x00, 0x00,
            0x00, 0x00, 0x00, 0x00,
            0x10, 0xc0, 0x41, 0xc7,
            0x40, 0x0c, 0x00, 0x00,
            0x00, 0x00, 0x49, 0x89,
            0x00, 0x48, 0x8b, 0x05,
             '?',  '?',  '?',  '?', // Run_Room*
            0x48, 0x85, 0xc0, 0x74,
            0x48, 0x85, 0xd2
        }
    );

    auto gpglobalsig = fastcodesearch(
        prochandle, procmodinfo, {
            // e8 ? ? ? ? 48 8b 3d ? ? ? ? 33 ed 48 8b c8 48 89 2b 89 6b 08
            0xe8,  '?',  '?',  '?',
             '?', 0x48, 0x8b, 0x3d,
             '?',  '?',  '?',  '?',
            0x33, 0xed, 0x48, 0x8b,
            0xc8, 0x48, 0x89, 0x2b,
            0x89, 0x6b, 0x08
        }
    );

    reader r(prochandle);

    auto instancevarlookupptr = varlookupsig + 11 + r.read<uint32_t>(varlookupsig + 7);
    auto instancevarlookup = r.read<LPBYTE>(instancevarlookupptr);
    //auto instancevarlookup = instancevarlookupptr;

    auto runroomptr = runroomsig + 28 + r.read<uint32_t>(runroomsig + 24);
    auto runroom = r.read<LPBYTE>(runroomptr);

    auto gpglobalptr = gpglobalsig + 12 + r.read<uint32_t>(gpglobalsig + 8);
    auto gpglobal = r.read<LPBYTE>(gpglobalptr);

    auto getvarquick = [&](LPBYTE instptr, const string& name) -> pair<LPBYTE, string> {
        auto slot = getvarslot(r, instancevarlookup, name);
        auto yyvarsmapoffs = 0x48;
        auto yyvarsmap = r.read<LPBYTE>(instptr + yyvarsmapoffs);
        auto rvptr = getvarbyslot(r, yyvarsmap, slot);
        if (rvptr == nullptr) {
            return make_pair(rvptr, "<nonexistant variable>");
        }
        auto flagsoffs = 8;
        auto kindoffs = 12;
        auto rkind = r.read<int>(rvptr + kindoffs) & 0x0ffffff;
        auto rflags = r.read<int>(rvptr + flagsoffs);
        if (rkind == 0) {
            return make_pair(rvptr, to_string(r.read<double>(rvptr)));
        }
        else if (rkind == 13) {
            return make_pair(rvptr, r.read<double>(rvptr) > 0.5 ? "true" : "false");
        }
        else if (rkind == 5) {
            return make_pair(rvptr, "undefined");
        }
        else if (rkind == 1) {
            auto refthing = r.read<LPBYTE>(rvptr);
            auto thingptr = r.read<LPBYTE>(refthing);
            auto contents = r.read<smallstr>(thingptr).c;
            return make_pair(rvptr, contents);
        }
    };

    instanceloop(r, runroom, [&](LPBYTE instptr, int idx) {
        auto idoffs = 0xb4;
        auto pobjectoffs = 0x90;
        auto instid = r.read<int>(instptr + idoffs);
        auto objptr = r.read<LPBYTE>(instptr + pobjectoffs); // CObjectGM*
        auto nameoffs = 0x00;
        auto objname = r.read<smallstr>(r.read<LPBYTE>(objptr + nameoffs)).c;
        cout << "instance " << idx << " = " << instid << " (" << objname << ")" << endl;
        if (string("obj_male") == objname) {
            cout << "obj_male.anim_suffix = " << getvarquick(instptr, "anim_suffix").second << endl;
        }
        return false; // keep looping...
    });

    auto devdebug = getvarquick(gpglobal, "dev_debug");
    cout << devdebug.second << endl;
    r.write(devdebug.first, 0.0 + !r.read<double>(devdebug.first));

    return EXIT_SUCCESS;
}

