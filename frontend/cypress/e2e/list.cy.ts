// Check got redirect when access to '/'
describe('Check redirect GET /', () => {
    it('check', () => {
        cy.visit('/')
        cy.url().should('include', '/list')
    })
})

describe('List view check', () => {
    it('check there are no parent dir button on root', () => {
        cy.visit('/list?path=%2F')
        cy.contains('Parent dir').should('not.exist');

        cy.visit('/list?path=')
        cy.contains('Parent dir').should('not.exist');

        cy.visit('/list')
        cy.contains('Parent dir').should('not.exist');
    })

    it('check nonexistent dir is not found', () => {
        cy.visit('/list?path=nonexistent')

        // Check it displays: Not found or error.
        cy.contains('Not found or error.').should('exist');
    });

    it('check tests dir exists', () => {
        cy.visit('/list?path=')
        cy.contains('tests').should('exist');
        cy.contains('tests').click();
        cy.url().should('include', '/list?path=tests')
    })

    it('check tests dir exists with non standard link', () => {
        cy.visit('/list')
        cy.contains('tests').should('exist');
        cy.contains('tests').click();
        cy.url().should('include', '/list?path=tests')

        cy.visit('/list?path=%2F')
        cy.contains('tests').should('exist');
        cy.contains('tests').click();
        // ToDo: Should normalize
        cy.url().should('include', '/list?path=%2Ftests')
    })

    it('check parent dir button works', () => {
        cy.visit('/list?path=tests')

        cy.contains('Parent dir').should('exist');

        cy.contains('Parent dir').click();
        cy.url().should('include', '/list?path=')
        cy.contains('Parent dir').should('not.exist');
    })

    it('check Testing Introduction 01.cbz exists', () => {
        cy.visit('/list?path=tests')

        cy.contains('Testing Introduction 01.cbz').should('exist');
    })

    it('check Testing Introduction 01.cbz thumbnail works', () => {
        cy.visit('/list?path=tests')
        cy.get('img[alt="Thumbnail of Testing Introduction 01.cbz"]')
            .should('be.visible')
            .and(($img) => {
                // Thumbnail's width should 100px. see: img.go
                expect($img[0].naturalWidth).to.be.equal(100);
                // Check aspect ratio is preserved
                expect($img[0].naturalHeight).to.be.equal(150);
            })
    })

    it('check Testing Introduction 01.cbz read url works', () => {
        cy.visit('/list?path=tests')
        cy.contains('Testing Introduction 01.cbz').click();

        cy.url().should('include', '/read?path=tests%2FTesting%20Introduction%2001.cbz');
    });
});

