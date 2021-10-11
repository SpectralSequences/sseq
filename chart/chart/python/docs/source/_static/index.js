document.addEventListener("keypress", (e)=> {
    if(document.activeElement.matches("input")){
        return;
    }
    if(e.key === "s"){
        document.querySelector("[role='search'] input").focus();
        e.preventDefault();
    }
});