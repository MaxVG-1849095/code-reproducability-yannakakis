select count(*) from imdb119, imdb44, imdb83 where imdb119.d = imdb44.s and imdb44.s = imdb83.s;