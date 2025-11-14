describe('Read view check', () => {
    it('Check back button works', () => {
        cy.visit('/read?path=tests%2FTesting%20Introduction%2001.cbz')
        cy.contains('Back').click();
        // ToDo: Normalize URL
        cy.url().should('include', '/list?path=/tests')
    })

    it('Check page 1 is visible', () => {
        cy.visit('/read?path=tests%2FTesting%20Introduction%2001.cbz')

        cy.get('a#pgNum').should('have.text', '1')


        // Check `Image of page 1` visible.
        cy.get('img[alt="Image of page 1"]').should('be.visible').and(($img) => {
            const rect = $img[0].getBoundingClientRect();
            expect(rect.top).to.be.greaterThan(0);
            expect(rect.bottom).to.be.lessThan(Cypress.config('viewportHeight'));
        });

        // Check `Image of page 2` exists but not visible due to requires scroll to see.
        cy.get('img[alt="Image of page 2"]').should('exist').and(($img) => {
            const rect = $img[0].getBoundingClientRect();
            expect(rect.bottom).to.be.greaterThan(Cypress.config('viewportHeight'));
        });

    })

    it('Check page hash works', () => {
        cy.visit('/read?path=tests%2FTesting%20Introduction%2001.cbz#3')

        cy.get('a#pgNum').should('have.text', '3')

        // Check `Image of page 1` exists but not visible due to requires scroll to see.
        cy.get('img[alt="Image of page 1"]').should('exist').and(($img) => {
            const rect = $img[0].getBoundingClientRect();
            // The image is scrolled out of view from the top, so its bottom should be less than 0.
            expect(rect.bottom).to.be.lessThan(0);
        });

        // Check `Image of page 3` visible.
        cy.get('img[alt="Image of page 3"]').should('be.visible').and(($img) => {
            const rect = $img[0].getBoundingClientRect();
            // The image is in view, so its top should be greater than 0 and its bottom
            // should be less than the viewport height.
            expect(rect.top).to.be.greaterThan(0);
            expect(rect.bottom).to.be.lessThan(Cypress.config('viewportHeight'));
        });
    })

    it('Check page Prev works on LtR', () => {
        cy.visit('/read?path=tests%2FTesting%20Introduction%2001.cbz#3')

        // If there are link `RtL`, Click. If there are no link `RtL`, Continue processing.
        // To make Left to Right mode.
        cy.get('body').then(($body) => {
            if ($body.find('a:contains("RtL")').length) {
                cy.contains('RtL').click();
            }
        });

        cy.contains('Prev').click();
        cy.get('a#pgNum').should('have.text', '2')

        // Check `Image of page 2` visible.
        cy.get('img[alt="Image of page 2"]').should('be.visible').and(($img) => {
            const rect = $img[0].getBoundingClientRect();
            expect(rect.top).to.be.greaterThan(0);
            expect(rect.bottom).to.be.lessThan(Cypress.config('viewportHeight'));
        });
    })

    it('Check page Next works on LtR', () => {
        cy.visit('/read?path=tests%2FTesting%20Introduction%2001.cbz#3')

        // If there are link `RtL`, Click. If there are no link `RtL`, Continue processing.
        // To make Left to Right mode.
        cy.get('body').then(($body) => {
            if ($body.find('a:contains("RtL")').length) {
                cy.contains('RtL').click();
            }
        });

        cy.contains('Next').click();
        cy.get('a#pgNum').should('have.text', '4')

        // Check `Image of page 2` visible.
        cy.get('img[alt="Image of page 4"]').should('be.visible').and(($img) => {
            const rect = $img[0].getBoundingClientRect();
            expect(rect.top).to.be.greaterThan(0);
            expect(rect.bottom).to.be.lessThan(Cypress.config('viewportHeight'));
        });
    })

    it('Check page Prev works on RtL', () => {
        cy.visit('/read?path=tests%2FTesting%20Introduction%2001.cbz#3')

        cy.get('body').then(($body) => {
            if ($body.find('a:contains("LtR")').length) {
                cy.contains('LtR').click();
            }
        });

        cy.contains('Prev').click();
        cy.get('a#pgNum').should('have.text', '2')

        // Check `Image of page 2` visible.
        cy.get('img[alt="Image of page 2"]').should('be.visible').and(($img) => {
            const rect = $img[0].getBoundingClientRect();
            expect(rect.top).to.be.greaterThan(0);
            expect(rect.bottom).to.be.lessThan(Cypress.config('viewportHeight'));
        });
    })

    it('Check page Next works on RtL', () => {
        cy.visit('/read?path=tests%2FTesting%20Introduction%2001.cbz#3')

        cy.get('body').then(($body) => {
            if ($body.find('a:contains("LtR")').length) {
                cy.contains('LtR').click();
            }
        });

        cy.contains('Next').click();
        cy.get('a#pgNum').should('have.text', '4')

        // Check `Image of page 2` visible.
        cy.get('img[alt="Image of page 4"]').should('be.visible').and(($img) => {
            const rect = $img[0].getBoundingClientRect();
            expect(rect.top).to.be.greaterThan(0);
            expect(rect.bottom).to.be.lessThan(Cypress.config('viewportHeight'));
        });
    })

    it('Check page set prompt', () => {
        cy.visit('/read?path=tests%2FTesting%20Introduction%2001.cbz')

        cy.window().then((w) => {
            cy.stub(w, 'prompt').returns('2')
        });

        cy.get('a#pgNum').click();

        cy.get('a#pgNum').should('have.text', '2')
        cy.get('img[alt="Image of page 2"]').should('be.visible').and(($img) => {
            const rect = $img[0].getBoundingClientRect();
            expect(rect.top).to.be.greaterThan(0);
            expect(rect.bottom).to.be.lessThan(Cypress.config('viewportHeight'));
        });
    })

    it('Check hash page no changes', () => {
        cy.visit('/read?path=tests%2FTesting%20Introduction%2001.cbz')
        cy.contains('Next').click();
        cy.url().should('include', '#2');
        cy.contains('Prev').click();
        cy.url().should('include', '#1');
    });
});
