//! Authoritative desktop projection and mutation lifecycle for user albums and
//! rebuildable album suggestions. Audio Space composes this owner but does not
//! reinterpret album evidence or persistence results.

import QtQuick
import EchoDesktop

QtObject {
    id: state

    required property var catalogBackend

    property var userAlbums: []
    property var suggestedAlbums: []
    property string errorMessage: ""

    signal albumsRefreshed()

    property Connections backendConnections: Connections {
        target: state.catalogBackend
        function onAlbumsChanged() : void { state.refresh() }
    }

    function refresh() : void {
        userAlbums = catalogBackend.listUserAlbums()
        suggestedAlbums = catalogBackend.listSmartAlbums()
        albumsRefreshed()
    }

    function userAlbum(albumId: var) : var {
        for (const album of userAlbums) {
            if (album.id === albumId) return album
        }
        return null
    }

    function suggestedAlbum(key: string) : var {
        for (const album of suggestedAlbums) {
            if (album.key === key) return album
        }
        return null
    }

    function createAlbum(name: string, memberIds: var) : int {
        errorMessage = ""
        const albumId = catalogBackend.createUserAlbum(name, memberIds || [])
        if (albumId < 0) {
            errorMessage = qsTr("The album could not be created. Its name may already be in use.")
        }
        return albumId
    }

    function renameAlbum(albumId: var, name: string) : bool {
        errorMessage = ""
        if (!catalogBackend.renameUserAlbum(albumId, name)) {
            errorMessage = qsTr("The album could not be renamed. Its name may already be in use.")
            return false
        }
        return true
    }

    function deleteAlbum(albumId: var) : bool {
        errorMessage = ""
        if (!catalogBackend.deleteUserAlbum(albumId)) {
            errorMessage = qsTr("The album could not be deleted.")
            return false
        }
        return true
    }

    function setMembership(albumId: var, assetId: string, included: bool) : bool {
        errorMessage = ""
        if (!catalogBackend.setUserAlbumMembership(albumId, assetId, included)) {
            errorMessage = qsTr("The sound could not be moved into the album.")
            return false
        }
        return true
    }
}
