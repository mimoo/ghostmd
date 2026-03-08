import XCTest

final class GhostMDUITests: XCTestCase {
    let app = XCUIApplication()

    override func setUpWithError() throws {
        continueAfterFailure = false
        app.launchArguments = ["--ui-testing"]
        app.launch()
    }

    // MARK: - Empty State

    func testEmptyStateShowsNoNotes() {
        XCTAssertTrue(app.staticTexts["No Notes"].waitForExistence(timeout: 5))
        XCTAssertTrue(app.buttons["composeButton"].exists)
    }

    func testTitleIsGhostMD() {
        XCTAssertTrue(app.navigationBars["GhostMD"].waitForExistence(timeout: 5))
    }

    // MARK: - New Note Sheet

    func testComposeButtonOpensNewNoteSheet() {
        app.buttons["composeButton"].tap()
        XCTAssertTrue(app.staticTexts["Where to create the note?"].waitForExistence(timeout: 3))
        XCTAssertTrue(app.buttons["newNoteDiaryButton"].exists)
        XCTAssertTrue(app.buttons["newNoteCurrentFolderButton"].exists)
        XCTAssertTrue(app.buttons["newNoteChooseFolderButton"].exists)
    }

    func testNewNoteSheetCancel() {
        app.buttons["composeButton"].tap()
        XCTAssertTrue(app.buttons["cancelButton"].waitForExistence(timeout: 3))
        app.buttons["cancelButton"].tap()
        // Sheet should dismiss, back to empty state
        XCTAssertTrue(app.staticTexts["No Notes"].waitForExistence(timeout: 3))
    }

    // MARK: - Create Note in Current Folder

    func testCreateNoteInCurrentFolder() {
        app.buttons["composeButton"].tap()
        XCTAssertTrue(app.buttons["newNoteCurrentFolderButton"].waitForExistence(timeout: 3))
        app.buttons["newNoteCurrentFolderButton"].tap()

        // Should navigate to editor
        let editor = app.textViews["noteEditor"]
        XCTAssertTrue(editor.waitForExistence(timeout: 5))
    }

    func testCreateNoteAndTypeText() {
        app.buttons["composeButton"].tap()
        app.buttons["newNoteCurrentFolderButton"].tap()

        let editor = app.textViews["noteEditor"]
        XCTAssertTrue(editor.waitForExistence(timeout: 5))
        editor.tap()
        editor.typeText("Hello from XCUITest!")

        // Verify text was typed
        XCTAssertTrue(editor.value as? String == "Hello from XCUITest!" ||
                      (editor.value as? String)?.contains("Hello from XCUITest!") == true)
    }

    // MARK: - Create Diary Note

    func testCreateDiaryNote() {
        app.buttons["composeButton"].tap()
        XCTAssertTrue(app.buttons["newNoteDiaryButton"].waitForExistence(timeout: 3))
        app.buttons["newNoteDiaryButton"].tap()

        let editor = app.textViews["noteEditor"]
        XCTAssertTrue(editor.waitForExistence(timeout: 5))
    }

    // MARK: - Navigation

    func testCreateNoteAndGoBack() {
        // Create a note
        app.buttons["composeButton"].tap()
        app.buttons["newNoteCurrentFolderButton"].tap()
        let editor = app.textViews["noteEditor"]
        XCTAssertTrue(editor.waitForExistence(timeout: 5))

        // Go back
        app.navigationBars.buttons.element(boundBy: 0).tap()

        // Should see the note in the list now (not empty state)
        XCTAssertFalse(app.staticTexts["No Notes"].waitForExistence(timeout: 3))
    }

    // MARK: - Editor Menu Actions

    func testEditorMenuShowsOptions() {
        app.buttons["composeButton"].tap()
        app.buttons["newNoteCurrentFolderButton"].tap()
        XCTAssertTrue(app.textViews["noteEditor"].waitForExistence(timeout: 5))

        // Dismiss keyboard first if needed
        if app.buttons["doneButton"].exists {
            app.buttons["doneButton"].tap()
        }

        app.buttons["menuButton"].tap()

        XCTAssertTrue(app.buttons["renameButton"].waitForExistence(timeout: 3))
        XCTAssertTrue(app.buttons["moveButton"].exists)
        XCTAssertTrue(app.buttons["deleteButton"].exists)
    }

    // MARK: - Delete Note

    func testDeleteNoteFromEditor() {
        // Create a note
        app.buttons["composeButton"].tap()
        app.buttons["newNoteCurrentFolderButton"].tap()
        XCTAssertTrue(app.textViews["noteEditor"].waitForExistence(timeout: 5))

        if app.buttons["doneButton"].exists {
            app.buttons["doneButton"].tap()
        }

        // Open menu and delete
        app.buttons["menuButton"].tap()
        app.buttons["deleteButton"].tap()

        // Confirm deletion — confirmationDialog can duplicate elements, use firstMatch
        let deleteConfirm = app.buttons["confirmDeleteButton"].firstMatch
        XCTAssertTrue(deleteConfirm.waitForExistence(timeout: 3))
        deleteConfirm.tap()

        // Should go back to folder view
        XCTAssertTrue(app.navigationBars["GhostMD"].waitForExistence(timeout: 5))
    }

    // MARK: - Choose Folder Flow

    func testChooseFolderShowsPicker() {
        app.buttons["composeButton"].tap()
        XCTAssertTrue(app.buttons["newNoteChooseFolderButton"].waitForExistence(timeout: 3))
        app.buttons["newNoteChooseFolderButton"].tap()

        // Folder picker should appear
        XCTAssertTrue(app.navigationBars["Choose Folder"].waitForExistence(timeout: 3))
        XCTAssertTrue(app.buttons["newFolderButton"].exists)
    }
}
