//! Desktop backend facade: the only Qt-owned bridge to the Rust memory
//! engine. QML never opens SQLite; all Library reads go through this object.

#pragma once

#include <QObject>
#include <QTimer>
#include <QUrl>
#include <QVariantList>
#include <QVariantMap>

#include "echo-desktop-bridge/src/lib.rs.h"
#include "rust/cxx.h"

class DesktopBackend : public QObject {
    Q_OBJECT
    Q_PROPERTY(quint64 assetCount READ assetCount NOTIFY assetsChanged)
    Q_PROPERTY(QString catalogPath READ catalogPath NOTIFY assetsChanged)
    Q_PROPERTY(QString cacheRoot READ cacheRoot NOTIFY assetsChanged)

  public:
    explicit DesktopBackend(
        rust::Box<echo::desktop::LibrarySession> session,
        QObject* parent = nullptr
    );
    ~DesktopBackend() override = default;

    Q_INVOKABLE void refresh();
    Q_INVOKABLE QVariantList listAssets(bool originals = false) const;
    Q_INVOKABLE QString
    setSoundMembership(const QString& id, bool memory, bool materials, const QString& category);
    Q_INVOKABLE QVariantList projectMaterials(const QString& assemblyId) const;
    Q_INVOKABLE QString importMaterial(
        const QUrl& file,
        const QString& assemblyId,
        bool global,
        const QString& category
    );
    [[nodiscard]] QVariantMap memoryOutputDestination(const QString& assemblyId) const;
    Q_INVOKABLE QVariantList listSoundAssemblies() const;
    Q_INVOKABLE QVariantMap
    createSoundAssembly(const QString& name, const QVariantList& assetIds, const QString& layout);
    Q_INVOKABLE QVariantMap soundAssembly(const QString& assemblyId) const;
    Q_INVOKABLE QVariantMap saveSoundAssembly(const QVariantMap& document);
    Q_INVOKABLE bool archiveSoundAssembly(const QString& assemblyId);
    Q_INVOKABLE QString newAssemblyObjectId() const;
    Q_INVOKABLE QVariantList listImpulseResponses() const;
    [[nodiscard]] QVariantMap importImpulseResponse(
        const QString& sourcePath,
        const QString& displayName,
        const QString& creator,
        const QString& sourceUrl,
        const QString& attribution,
        const QString& rightsKind,
        const QString& spdxExpression,
        const QString& licenseUrl
    ) const;
    [[nodiscard]] QVariantMap importImpulseResponseWithLayout(
        const QString& sourcePath,
        const QString& preparationLayout,
        const QString& displayName,
        const QString& creator,
        const QString& sourceUrl,
        const QString& attribution,
        const QString& rightsKind,
        const QString& spdxExpression,
        const QString& licenseUrl
    ) const;
    Q_INVOKABLE QVariantList listKeywordFacets() const;
    Q_INVOKABLE QVariantList listSmartAlbums() const;
    Q_INVOKABLE QVariantList listUserAlbums() const;
    Q_INVOKABLE QVariantMap revisitSnapshot() const;
    Q_INVOKABLE QVariantList listProcessingRecipes() const;
    Q_INVOKABLE QVariantList listProcessingRecipeHistory() const;
    Q_INVOKABLE QString createProcessingRecipe(
        const QString& name,
        const QString& sourceAssetId,
        const QVariantList& componentIds
    );
    Q_INVOKABLE bool renameProcessingRecipe(const QString& recipeId, const QString& name);
    Q_INVOKABLE qlonglong updateProcessingRecipe(
        const QString& recipeId,
        const QString& sourceAssetId,
        const QVariantList& componentIds
    );
    Q_INVOKABLE bool archiveProcessingRecipe(const QString& recipeId);
    Q_INVOKABLE QVariantMap applyProcessingRecipe(
        const QString& recipeId,
        const QVariantList& targetAssetIds,
        const QString& mergeMode
    );
    Q_INVOKABLE QVariantMap revertProcessingRecipeApplication(const QString& batchId);
    Q_INVOKABLE qlonglong createUserAlbum(const QString& name, const QVariantList& memberIds);
    Q_INVOKABLE bool renameUserAlbum(qlonglong albumId, const QString& name);
    Q_INVOKABLE bool deleteUserAlbum(qlonglong albumId);
    Q_INVOKABLE bool
    setUserAlbumMembership(qlonglong albumId, const QString& assetId, bool included);
    Q_INVOKABLE QVariantList waveformForAsset(const QString& id) const;
    Q_INVOKABLE QVariantList transcriptsForAsset(const QString& id) const;
    Q_INVOKABLE QVariantList longAudioChaptersForAsset(const QString& id) const;
    Q_INVOKABLE bool setAssetAffinity(const QString& id, bool liked, int rating);
    Q_INVOKABLE QVariantMap calibrateAssetMetadata(
        const QString& id,
        const QString& soundCaption,
        const QString& summary,
        const QString& eventType,
        const QString& mood,
        const QVariantList& keywords,
        const QString& transcriptText,
        const QString& language,
        const QVariantList& calibratedFields
    );
    Q_INVOKABLE QVariantMap recordListeningProgress(
        const QString& id,
        qlonglong positionMillis,
        qlonglong playbackStartMillis,
        qlonglong playbackEndMillis
    );
    // Preserves the existing QML save call until the Creative draft is connected.
    Q_INVOKABLE bool setAssetAdjustment(
        const QString& id,
        qlonglong trimStartMillis,
        qlonglong trimEndMillis,
        qlonglong fadeInMillis,
        qlonglong fadeOutMillis,
        int fadeInCurve,
        int fadeOutCurve,
        int gainCentibels,
        int lowCutHertz,
        bool restorationEnabled,
        bool dePlosiveEnabled,
        int dePlosiveFrequencyHertz,
        int dePlosiveSensitivityPercent,
        int dePlosiveReductionCentibels,
        int dePlosiveReleaseMillis,
        bool noiseReductionEnabled,
        int noiseReductionCentibels,
        int noiseReductionSensitivityPercent,
        int noiseReductionSmoothingMillis,
        bool deEsserEnabled,
        int deEsserFrequencyHertz,
        int deEsserThresholdCentibels,
        int deEsserReductionCentibels,
        bool deHumEnabled,
        int deHumFundamentalHertz,
        int deHumHarmonicCount,
        int deHumQualityTenths,
        int deHumDepthCentibels,
        bool deClickEnabled,
        int deClickSensitivityPercent,
        int deClickMaximumClickMicroseconds,
        int deClickRepairPercent,
        bool channelRepairEnabled,
        bool channelRepairInvertLeft,
        bool channelRepairInvertRight,
        bool channelRepairSwapChannels,
        bool channelRepairMonoFoldDown,
        int channelRepairBalancePercent,
        bool equalizerEnabled,
        const QVariantList& equalizerBands,
        bool compressorEnabled,
        int compressorThresholdCentibels,
        int compressorRatioTenths,
        int compressorAttackMillis,
        int compressorReleaseMillis,
        int compressorMakeupCentibels,
        int reverbCharacter,
        bool reverbEnabled,
        int reverbMixPercent,
        int reverbPreDelayMillis,
        int reverbDecayMillis,
        int reverbSizePercent,
        int reverbDampingPercent,
        int reverbLowCutHertz,
        int reverbHighCutHertz,
        bool limiterEnabled,
        int limiterCeilingCentibels,
        int limiterReleaseMillis,
        const QVariantList& effectChain,
        const QVariantList& editSegments,
        const QVariantList& effectMasks
    );
    Q_INVOKABLE bool setAssetAdjustment(
        const QString& id,
        qlonglong trimStartMillis,
        qlonglong trimEndMillis,
        qlonglong fadeInMillis,
        qlonglong fadeOutMillis,
        int fadeInCurve,
        int fadeOutCurve,
        int gainCentibels,
        int lowCutHertz,
        bool restorationEnabled,
        bool dePlosiveEnabled,
        int dePlosiveFrequencyHertz,
        int dePlosiveSensitivityPercent,
        int dePlosiveReductionCentibels,
        int dePlosiveReleaseMillis,
        bool noiseReductionEnabled,
        int noiseReductionCentibels,
        int noiseReductionSensitivityPercent,
        int noiseReductionSmoothingMillis,
        bool deEsserEnabled,
        int deEsserFrequencyHertz,
        int deEsserThresholdCentibels,
        int deEsserReductionCentibels,
        bool deHumEnabled,
        int deHumFundamentalHertz,
        int deHumHarmonicCount,
        int deHumQualityTenths,
        int deHumDepthCentibels,
        bool deClickEnabled,
        int deClickSensitivityPercent,
        int deClickMaximumClickMicroseconds,
        int deClickRepairPercent,
        bool channelRepairEnabled,
        bool channelRepairInvertLeft,
        bool channelRepairInvertRight,
        bool channelRepairSwapChannels,
        bool channelRepairMonoFoldDown,
        int channelRepairBalancePercent,
        bool equalizerEnabled,
        const QVariantList& equalizerBands,
        bool compressorEnabled,
        int compressorThresholdCentibels,
        int compressorRatioTenths,
        int compressorAttackMillis,
        int compressorReleaseMillis,
        int compressorMakeupCentibels,
        int reverbCharacter,
        bool reverbEnabled,
        int reverbMixPercent,
        int reverbPreDelayMillis,
        int reverbDecayMillis,
        int reverbSizePercent,
        int reverbDampingPercent,
        int reverbLowCutHertz,
        int reverbHighCutHertz,
        bool limiterEnabled,
        int limiterCeilingCentibels,
        int limiterReleaseMillis,
        const QVariantList& effectChain,
        const QVariantList& editSegments,
        const QVariantList& effectMasks,
        const QVariantMap& creativeVfx,
        const QVariantMap& spectralRepair,
        const QVariantMap& space,
        const QVariantMap& projectDocument = {},
        const QString& projectClipId = {}
    );
    Q_INVOKABLE QVariantList search(const QString& query) const;
    Q_INVOKABLE QVariantMap analysisStatusForAsset(const QString& id) const;
    Q_INVOKABLE QVariantList analysisStatuses() const;
    Q_INVOKABLE bool retryAnalysis(const QString& id);
    Q_INVOKABLE qulonglong retryFailedAnalysis();
    void startWorkers(const QString& runtimeEndpoint);
    Q_INVOKABLE void queueScans();
    Q_INVOKABLE QVariantMap jobStats() const;
    Q_INVOKABLE QVariantList listRoots() const;
    Q_INVOKABLE bool addRoot(const QUrl& folder);
    Q_INVOKABLE void removeRoot(qlonglong id);
    quint64 assetCount() const;
    QString catalogPath() const;
    QString cacheRoot() const;
    /// Worker-thread publication handoff after an atomic render commit.
    /// Returns an empty string on success or a localized-ready technical
    /// detail for the controller to present.
    [[nodiscard]] QString recordRenderExport(
        const QString& assetId,
        qint64 adjustmentRevisionId,
        const QString& outputPath,
        const QString& format,
        quint32 sampleRate,
        quint32 channelCount,
        quint16 bitDepth,
        quint64 frameCount,
        quint64 sizeBytes,
        float integratedLufs,
        float truePeakDbtp
    ) const;
    [[nodiscard]] QString recordSoundAssemblyExport(
        const QString& assemblyId,
        qint64 assemblyRevisionId,
        const QString& outputPath,
        quint32 sampleRate,
        quint32 channelCount,
        quint16 bitDepth,
        quint64 frameCount,
        quint64 sizeBytes,
        float integratedLufs,
        float truePeakDbtp,
        bool preserveMemory = false
    ) const;
    /// Records a user delivery from a verified post-effect repair copy and
    /// persists an immutable snapshot of that copy's current provenance.
    [[nodiscard]] QString recordRenderedSpectralWorkingCopyExport(
        const QString& assetId,
        qint64 adjustmentRevisionId,
        qint64 workingCopyId,
        const QString& renderedSourcePath,
        const QString& outputPath,
        const QString& format,
        quint32 sampleRate,
        quint32 channelCount,
        quint16 bitDepth,
        quint64 frameCount,
        quint64 sizeBytes,
        float integratedLufs,
        float truePeakDbtp
    ) const;
    /// Worker-thread admission after a private full render completes. The
    /// returned map names only a verified content-addressed cache path.
    [[nodiscard]] QVariantMap createRenderedSpectralWorkingCopy(
        const QString& assetId,
        qint64 adjustmentRevisionId,
        const QString& renderedPath
    ) const;
    /// Worker-thread commit after one deterministic erase render. The cache
    /// payload and the shared working-layer manifest advance together.
    [[nodiscard]] QVariantMap commitRenderedSpectralErase(
        const QString& assetId,
        qint64 workingCopyId,
        const QString& renderedPath,
        quint64 startMillis,
        quint64 endMillis,
        quint16 lowHertz,
        quint16 highHertz,
        qint16 attenuationCentibels,
        quint16 timeFeatherMillis,
        quint16 frequencyFeatherHertz
    ) const;
    Q_INVOKABLE QVariantList renderedSpectralWorkingCopies(const QString& assetId) const;
    Q_INVOKABLE bool setRenderedSpectralWorkingCopyEnabled(
        const QString& assetId,
        qint64 workingCopyId,
        bool enabled
    ) const;
    Q_INVOKABLE bool
    removeRenderedSpectralWorkingCopy(const QString& assetId, qint64 workingCopyId) const;

  signals:
    void projectClipSaved(const QVariantMap& revision);
    void adjustmentSaveFailed(const QString& message);
    void assetsChanged();
    void soundAssembliesChanged();
    void albumsChanged();
    void processingRecipesChanged();
    void jobsChanged();
    void listeningStateChanged(
        const QString& id,
        qlonglong lastListenedAtMillis,
        qlonglong resumePositionMillis
    );
    void impulseResponsesChanged();

  private:
    rust::Box<echo::desktop::LibrarySession> session_;
    QTimer analysisRefreshTimer_;
    quint64 workerStateRevision_ = 0;
};
