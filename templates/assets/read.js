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
