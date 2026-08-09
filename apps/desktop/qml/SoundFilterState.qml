//! Existing-value filter projection for Audio Space. Values are derived from
//! the current Library snapshot; one value may match inside a facet while all
//! active facet families must match the sound.

import QtQuick

QtObject {
    id: state

    required property var assets

    property var selectedKeywords: []
    property var selectedMoods: []
    property var selectedYears: []
    property var selectedLocations: []
    property var selectedEvents: []

    readonly property var keywordOptions: buildOptions("keyword")
    readonly property var moodOptions: buildOptions("mood")
    readonly property var yearOptions: buildOptions("year")
    readonly property var locationOptions: buildOptions("location")
    readonly property var eventOptions: buildOptions("event")
    readonly property int activeCount: selectedKeywords.length
        + selectedMoods.length + selectedYears.length
        + selectedLocations.length + selectedEvents.length

    signal filtersChanged()

    function normalize(value: var) : string {
        return String(value || "").replace(/\s+/g, " ").trim().toLocaleLowerCase()
    }

    function valuesFor(asset: var, facet: string) : var {
        if (facet === "keyword") {
            const values = []
            for (let index = 0; index < asset.keywords.length; ++index) {
                values.push(String(asset.keywords[index]))
            }
            return values
        }
        if (facet === "mood") {
            return asset.mood.length > 0 ? [asset.mood] : []
        }
        if (facet === "location") {
            return asset.sourceLocation.length > 0 ? [asset.sourceLocation] : []
        }
        if (facet === "event") {
            return asset.eventType.length > 0 ? [asset.eventType] : []
        }
        if (facet === "year" && asset.recordedAtMillis > 0) {
            return [String(new Date(asset.recordedAtMillis).getFullYear())]
        }
        return []
    }

    function appendOption(options: var, key: string, label: string) : void {
        for (let index = 0; index < options.length; ++index) {
            if (options[index].key === key) {
                options[index].count += 1
                return
            }
        }
        options.push({ key: key, label: label, count: 1 })
    }

    function buildOptions(facet: string) : var {
        const options = []
        for (const asset of assets) {
            const values = valuesFor(asset, facet)
            const seen = []
            for (const rawValue of values) {
                const key = normalize(rawValue)
                if (key.length === 0 || seen.includes(key)) {
                    continue
                }
                seen.push(key)
                appendOption(options, key, String(rawValue).trim())
            }
        }
        options.sort((left, right) => right.count - left.count
            || left.label.localeCompare(right.label))
        return options
    }

    function selectedFor(facet: string) : var {
        if (facet === "keyword") return selectedKeywords
        if (facet === "mood") return selectedMoods
        if (facet === "year") return selectedYears
        if (facet === "location") return selectedLocations
        if (facet === "event") return selectedEvents
        return []
    }

    function replaceSelected(facet: string, values: var) : void {
        if (facet === "keyword") selectedKeywords = values
        else if (facet === "mood") selectedMoods = values
        else if (facet === "year") selectedYears = values
        else if (facet === "location") selectedLocations = values
        else if (facet === "event") selectedEvents = values
    }

    function toggle(facet: string, key: string) : void {
        const values = selectedFor(facet).slice()
        const index = values.indexOf(key)
        if (index >= 0) {
            values.splice(index, 1)
        } else {
            values.push(key)
        }
        replaceSelected(facet, values)
        filtersChanged()
    }

    function clear() : void {
        selectedKeywords = []
        selectedMoods = []
        selectedYears = []
        selectedLocations = []
        selectedEvents = []
        filtersChanged()
    }

    function facetMatches(asset: var, facet: string, selected: var) : bool {
        if (selected.length === 0) {
            return true
        }
        const values = valuesFor(asset, facet)
        for (const value of values) {
            if (selected.includes(normalize(value))) {
                return true
            }
        }
        return false
    }

    function matches(asset: var) : bool {
        return facetMatches(asset, "keyword", selectedKeywords)
            && facetMatches(asset, "mood", selectedMoods)
            && facetMatches(asset, "year", selectedYears)
            && facetMatches(asset, "location", selectedLocations)
            && facetMatches(asset, "event", selectedEvents)
    }
}
