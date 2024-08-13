const pgNum = document.getElementById('pgNum');

function chPageDec() {
    const page = parseInt(pgNum.innerText);
    if (page <= 1)
        return;
    document.getElementById((page - 1).toString()).scrollIntoView();
    pgNum.innerText = (page - 1).toString();
}

function chPageInc() {
    const page = parseInt(pgNum.innerText);
    if (page >= pageCnt)
    return;
    document.getElementById((page + 1).toString()).scrollIntoView();
    pgNum.innerText = (page + 1).toString();
}

window.onload = () => {
    document.onkeydown = (e) => {
        switch(e.key)
        {
            case 'ArrowLeft':
            case 'ArrowUp':
            case 'PageUp':
                chPageDec();
                break;

            case 'ArrowRight':
            case 'ArrowDown':
            case 'PageDown':
                chPageInc();
                break;
        }
    }
}
