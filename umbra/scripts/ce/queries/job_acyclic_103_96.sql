select count(*) from imdb119, imdb87, imdb39 where imdb119.d = imdb87.s and imdb87.s = imdb39.s;