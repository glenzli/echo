//! Desktop backend facade: the only Qt-owned bridge to the Rust memory
//! engine. QML never opens SQLite; all Library reads go through this object.

#pragma once

#include <QObject>
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
    Q_INVOKABLE QVariantList listAssets() const;
    Q_INVOKABLE QVariantList listKeywordFacets() const;
    Q_INVOKABLE QVariantList listSmartAlbums() const;
    Q_INVOKABLE QVariantList listUserAlbums() const;
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

  signals:
    void assetsChanged();
    void albumsChanged();
    void processingRecipesChanged();
    void jobsChanged();

  private:
    rust::Box<echo::desktop::LibrarySession> session_;
};
