exports.loadFromServer = async function(path){
    let response = await fetch(path);
    return await response.json();
};