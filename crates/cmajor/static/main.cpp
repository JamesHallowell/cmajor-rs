#include <cmajor/COM/cmaj_Library.h>

extern "C" cmaj::Library::EntryPoints *cmajor_getEntryPointsStatic() {
  struct EntryPointsImpl : public cmaj::Library::EntryPoints {
    EntryPointsImpl() = default;
    virtual ~EntryPointsImpl() = default;

    inline const char *getVersion() override {
      return cmaj::Library::getVersion();
    }

    cmaj::ProgramInterface *createProgram() override {
      return cmaj::Library::createProgram().getWithIncrementedRefCount();
    }

    const char *getEngineTypes() override {
      return cmaj::Library::getEngineTypes();
    }

    cmaj::EngineFactoryInterface *
    createEngineFactory(const char *name) override {
      return cmaj::Library::createEngineFactory(name)
          .getWithIncrementedRefCount();
    }
  };

  static EntryPointsImpl ep;
  return std::addressof(ep);
}
