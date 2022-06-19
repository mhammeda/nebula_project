<template>
    <v-card max-width="1000" color="primary" elevation="0">
        <v-app-bar elevation="0" shaped color="secondary">
            <v-toolbar-title class="white--text"> All Posts </v-toolbar-title>

            <v-spacer> </v-spacer>

            <v-menu offset-y :close-on-content-click="false" left>
                <template v-slot:activator="{ on, attrs }">
                    <v-btn icon v-bind="attrs" v-on="on" color="white">
                        <v-icon> mdi-tune </v-icon>
                    </v-btn>
                </template>
                <v-card width="200">
                    <v-card-title>Filters</v-card-title>
                    <v-divider></v-divider>
                </v-card>
            </v-menu>

            <v-menu open-on-hover top offset-y>
                <template v-slot:activator="{ on, attrs }">
                    <v-btn color="secondary" dark v-bind="attrs" v-on="on">
                        Sort by
                    </v-btn>
                </template>

                <v-list>
                    <v-list-item
                        v-for="(item, index) in filterOptions"
                        :key="index"
                    >
                        <v-list-item-title v-on:click="item.associatedFunction"
                            >{{ item.title }}
                        </v-list-item-title>
                    </v-list-item>
                </v-list>
            </v-menu>
        </v-app-bar>
        <div class="listOfPosts">
            <div v-if="postsLoading">
                <div v-for="index in 5" :key="index">
                    <v-skeleton-loader
                        type="article, actions"
                    ></v-skeleton-loader>
                </div>
            </div>
            <ul v-if="allPosts.length > 0">
                <div v-for="item of postsShown" v-bind:key="item.id">
                    <Post v-on:deletePost="getPosts" :item="item" />
                </div>
            </ul>
        </div>
        <v-card-actions>
            <v-spacer></v-spacer>
            <v-btn
                v-if="numbDisplayed < this.allPosts.length"
                text
                @click="showMore"
                >show more</v-btn
            >
            <v-card-text
                justify="center"
                align="center"
                class="text-overline"
                v-else
            >
                no more posts to show
            </v-card-text>
            <v-spacer></v-spacer>
        </v-card-actions>
        <v-snackbar v-model="errorSnackbar">
            {{ errorMessage }}

            <template v-slot:action="{ attrs }">
                <v-btn
                    color="red"
                    text
                    v-bind="attrs"
                    @click="errorSnackbar = false"
                >
                    Close
                </v-btn>
            </template>
        </v-snackbar>
    </v-card>
</template>

<script>
import Post from './post.vue'

export default {
    mounted() {
        this.getPosts()
    },
    components: {
        Post,
    },
    data() {
        return {
            userUrl: '/internal/users/',
            postsUrl: '/internal/posts',
            allPosts: [],

            // showing posts
            postsLoading: true,
            numbDisplayed: 25, // show 25 max by default
            postsShown: [],

            filterOptions: [
                {
                    title: 'Date and time',
                    associatedFunction: () => {
                        this.sortPostsByTime()
                    },
                },
                {
                    title: 'Title',
                    associatedFunction: () => {
                        this.sortPostsByTitle()
                    },
                },
                {
                    title: 'Host',
                    associatedFunction: () => {
                        this.sortPostsByHost()
                    },
                },
                {
                    title: 'Username',
                    associatedFunction: () => {
                        this.sortPostsByUser()
                    },
                },
                {
                    title: 'Content',
                    associatedFunction: () => {
                        this.sortPostsByContent()
                    },
                },
            ],

            // error handling
            errorSnackbar: false,
            errorMessage: '',
        }
    },
    methods: {
        getPosts() {
            // for now send http request for all posts.
            this.$http
                .get(this.postsUrl, {}, { withCredentials: true })
                .then(response => {
                    // check we got posts in response
                    if (response.data.length > 0) {
                        const posts = response.data
                        this.allPosts = posts

                        // sort posts by most recent
                        this.sortPostsByTime()
                        console.log(this.allPosts.length)

                        // show first numbDisplayed posts
                        if (this.allPosts.length > this.numbDisplayed) {
                            this.postsShown = posts.slice(0, this.numbDisplayed)
                        } else {
                            this.postsShown = posts
                        }

                        this.postsLoading = false
                        this.errorMessage = ''
                    } else {
                        this.postsLoading = false
                        this.errorMessage = 'No Posts to show :('
                    }
                })
                .catch(error => {
                    this.postsLoading = false
                    this.errorMessage = 'Problem contacting the server :('
                    this.errorSnackbar = true
                })
        },

        showMore() {
            if (this.numbDisplayed < this.allPosts.length) {
                // add on remaining posts or 25 more (whichever is smallest)
                if (this.allPosts.length > this.numbDisplayed + 25) {
                    this.numbDisplayed = this.numbDisplayed + 25
                    this.postsShown = this.allPosts.slice(0, this.numbDisplayed)
                } else {
                    this.numbDisplayed = this.allPosts.length
                    this.postsShown = this.allPosts.slice()
                }
            }
        },

        sortPostsByTime() {
            this.allPosts.sort((a, b) => {
                return b.created - a.created
            })
        },

        sortPostsByTitle() {
            this.allPosts.sort((a, b) => {
                if (a.title > b.title) {
                    return 1
                } else {
                    return -1
                }
            })
        },

        sortPostsByHost() {
            this.allPosts.sort((a, b) => {
                if (a.author.host > b.author.host) {
                    if (a.author.username > a.author.username) {
                        return 1
                    } else {
                        return -1
                    }
                } else {
                    return -1
                }
            })
        },

        sortPostsByUser() {
            this.allPosts.sort((a, b) => {
                if (a.username > b.username) {
                    return 1
                } else {
                    return -1
                }
            })
        },

        sortPostsByContent() {
            this.allPosts.sort((a, b) => {
                if (a.content[0].text.text > b.content[0].text.text) {
                    return 1
                } else {
                    return -1
                }
            })
        },

        sortPostsByLikes() {
            /** To be changed and deployed
            allPosts.sort((a,b) => {
                if (a.likes > b.likes) {
                    return 1
                } else {
                    return -1
                }
            })
            */
        },
    },
}
</script>

<style scoped>
.postsArea {
    margin: 1% 5% 0% 5%;
    overflow: auto;
}
.createPostButton {
    text-align: left;
    margin-left: 30px;
}
</style>
