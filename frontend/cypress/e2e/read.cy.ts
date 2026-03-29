// Helper: visit the reader with pre-set localStorage values so mode and direction
// are already applied before the Vue app initialises (avoids page-number side-effects
// from the UI-based mode-switch flow).
interface VisitOpts {
    hash?: string;
    pageMode?: 'single' | 'double' | 'double-except-first';
    isRtL?: boolean;
}
const visitReader = ({ hash, pageMode, isRtL }: VisitOpts = {}) => {
    const url = `/read?path=tests%2FTesting%20Introduction%2001.cbz${hash ? `#${hash}` : ''}`;
    cy.visit(url, {
        onBeforeLoad(win) {
            if (pageMode) win.localStorage.setItem('pageMode', pageMode);
            if (isRtL !== undefined) win.localStorage.setItem('isRtL', isRtL ? 'true' : 'false');
        },
    });
};

describe('Read view check', () => {
    beforeEach(() => {
        cy.clearLocalStorage();
    });

    it('Check back button works', () => {
        cy.visit('/read?path=tests%2FTesting%20Introduction%2001.cbz');
        cy.contains('Back').click();
        cy.url().should('include', '/list?path=tests');
    });

    it('Check page hash works', () => {
        cy.visit('/read?path=tests%2FTesting%20Introduction%2001.cbz#3');
        cy.get('a#pgNum').should('have.text', '3');
    });

    it('Check page set prompt', () => {
        cy.visit('/read?path=tests%2FTesting%20Introduction%2001.cbz');
        cy.window().then((w) => {
            cy.stub(w, 'prompt').returns('2');
        });
        cy.get('a#pgNum').click();
        cy.get('a#pgNum').should('have.text', '2');
    });

    it('Check hash page no changes', () => {
        cy.visit('/read?path=tests%2FTesting%20Introduction%2001.cbz');
        cy.contains('Next').click();
        cy.url().should('include', '#2');
        cy.contains('Prev').click();
        cy.url().should('include', '#1');
    });

    // ---------------------------------------------------------------------------
    // Single page mode
    // Each spread shows exactly one image. Navigation advances/retreats by 1 page.
    // ---------------------------------------------------------------------------
    describe('Single page mode', () => {
        it('shows one single-page image', () => {
            visitReader({ hash: '1', pageMode: 'single' });
            cy.get('a#pgNum').should('have.text', '1');
            cy.get('.page-img-container img.single-page').should('have.length', 1).and('be.visible');
        });

        it('LtR: Next increments page by 1', () => {
            visitReader({ hash: '2', pageMode: 'single' });
            cy.contains('Next').click();
            cy.get('a#pgNum').should('have.text', '3');
        });

        it('LtR: Prev decrements page by 1', () => {
            visitReader({ hash: '3', pageMode: 'single' });
            cy.contains('Prev').click();
            cy.get('a#pgNum').should('have.text', '2');
        });

        it('RtL: Next increments page by 1', () => {
            visitReader({ hash: '2', pageMode: 'single', isRtL: true });
            cy.contains('Next').click();
            cy.get('a#pgNum').should('have.text', '3');
        });

        it('RtL: Prev decrements page by 1', () => {
            visitReader({ hash: '3', pageMode: 'single', isRtL: true });
            cy.contains('Prev').click();
            cy.get('a#pgNum').should('have.text', '2');
        });
    });

    // ---------------------------------------------------------------------------
    // Double page mode
    // Each spread shows two images side-by-side. Navigation advances/retreats by 2.
    // When switching to this mode from an odd page (via UI), the page advances to
    // the next even page so that spreads always start on an even number.
    // ---------------------------------------------------------------------------
    describe('Double page mode', () => {
        it('shows two double-page images', () => {
            visitReader({ hash: '2', pageMode: 'double' });
            cy.get('a#pgNum').should('have.text', '2');
            cy.get('.page-img-container img.double-page').should('have.length', 2);
        });

        it('advances odd starting page to even when switching mode via UI', () => {
            // Visiting page 1 in single mode, then clicking the mode button switches
            // to double and automatically advances page 1 (odd) to page 2 (even).
            visitReader({ hash: '1', pageMode: 'single' });
            cy.contains(/^single$/).click(); // single → double
            cy.get('a#pgNum').should('have.text', '2');
        });

        it('LtR: Next increments page by 2', () => {
            visitReader({ hash: '2', pageMode: 'double' });
            cy.contains('Next').click();
            cy.get('a#pgNum').should('have.text', '4');
        });

        it('LtR: Prev decrements page by 2', () => {
            visitReader({ hash: '4', pageMode: 'double' });
            cy.contains('Prev').click();
            cy.get('a#pgNum').should('have.text', '2');
        });

        it('RtL: Next increments page by 2', () => {
            visitReader({ hash: '2', pageMode: 'double', isRtL: true });
            cy.contains('Next').click();
            cy.get('a#pgNum').should('have.text', '4');
        });

        it('RtL: Prev decrements page by 2', () => {
            visitReader({ hash: '4', pageMode: 'double', isRtL: true });
            cy.contains('Prev').click();
            cy.get('a#pgNum').should('have.text', '2');
        });

        it('LtR: first page is on the left, second on the right', () => {
            visitReader({ hash: '2', pageMode: 'double' });
            cy.get('.page-img-container img.double-page').then(($imgs) => {
                const left = $imgs[0].getBoundingClientRect();
                const right = $imgs[1].getBoundingClientRect();
                expect(left.left).to.be.lessThan(right.left);
            });
        });

        it('RtL: images are displayed side by side (second logical page on the left)', () => {
            // In RtL the template renders showingPages[1] first (left) and
            // showingPages[0] second (right), so the first DOM child is still left.
            visitReader({ hash: '2', pageMode: 'double', isRtL: true });
            cy.get('.page-img-container img.double-page').then(($imgs) => {
                const first = $imgs[0].getBoundingClientRect();
                const second = $imgs[1].getBoundingClientRect();
                // Both images should sit in the same horizontal row.
                expect(first.top).to.be.closeTo(second.top, 1);
                // The first DOM child is rendered to the left.
                expect(first.left).to.be.lessThan(second.left);
            });
        });
    });

    // ---------------------------------------------------------------------------
    // Double-except-first page mode
    // Page 1 is alone (shown as blank slot + page 1).
    // Subsequent spreads pair pages: [2,3], [4,5], ...
    // Navigation from page 1 advances by 1; all others advance/retreat by 2.
    // ---------------------------------------------------------------------------
    describe('Double-except-first page mode', () => {
        it('page 1 shows two double-page images (blank slot + first page)', () => {
            visitReader({ hash: '1', pageMode: 'double-except-first' });
            cy.get('a#pgNum').should('have.text', '1');
            cy.get('.page-img-container img.double-page').should('have.length', 2);
        });

        it('LtR: Next from page 1 goes to page 2 (increment by 1)', () => {
            visitReader({ hash: '1', pageMode: 'double-except-first' });
            cy.contains('Next').click();
            cy.get('a#pgNum').should('have.text', '2');
        });

        it('LtR: Next from page 2 goes to page 4 (increment by 2)', () => {
            visitReader({ hash: '2', pageMode: 'double-except-first' });
            cy.contains('Next').click();
            cy.get('a#pgNum').should('have.text', '4');
        });

        it('LtR: Prev from page 2 goes to page 1 (decrement by 1)', () => {
            visitReader({ hash: '2', pageMode: 'double-except-first' });
            cy.contains('Prev').click();
            cy.get('a#pgNum').should('have.text', '1');
        });

        it('LtR: Prev from page 4 goes to page 2 (decrement by 2)', () => {
            visitReader({ hash: '4', pageMode: 'double-except-first' });
            cy.contains('Prev').click();
            cy.get('a#pgNum').should('have.text', '2');
        });

        it('RtL: Next from page 1 goes to page 2', () => {
            visitReader({ hash: '1', pageMode: 'double-except-first', isRtL: true });
            cy.contains('Next').click();
            cy.get('a#pgNum').should('have.text', '2');
        });

        it('RtL: Prev from page 2 goes to page 1', () => {
            visitReader({ hash: '2', pageMode: 'double-except-first', isRtL: true });
            cy.contains('Prev').click();
            cy.get('a#pgNum').should('have.text', '1');
        });

        it('RtL: Next from page 2 goes to page 4 (increment by 2)', () => {
            visitReader({ hash: '2', pageMode: 'double-except-first', isRtL: true });
            cy.contains('Next').click();
            cy.get('a#pgNum').should('have.text', '4');
        });

        it('RtL: Prev from page 4 goes to page 2 (decrement by 2)', () => {
            visitReader({ hash: '4', pageMode: 'double-except-first', isRtL: true });
            cy.contains('Prev').click();
            cy.get('a#pgNum').should('have.text', '2');
        });

        it('page 2 shows two double-page images', () => {
            visitReader({ hash: '2', pageMode: 'double-except-first' });
            cy.get('.page-img-container img.double-page').should('have.length', 2);
        });
    });
});
